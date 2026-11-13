//! Window platform helpers (Windows DWM composition and transparency).

#[allow(unsafe_code)]
#[cfg(windows)]
fn find_main_hwnd() -> Option<windows::Win32::Foundation::HWND> {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongW, GetWindowThreadProcessId, GWL_STYLE, IsWindowVisible,
        WS_POPUP,
    };

    static FOUND: AtomicIsize = AtomicIsize::new(0);
    FOUND.store(0, Ordering::Relaxed);
    let current_pid = std::process::id();

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let current_pid = lparam.0 as u32;
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == current_pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
            if (style & WS_POPUP.0) != 0 {
                FOUND.store(hwnd.0 as isize, Ordering::Relaxed);
                return false.into();
            }
        }
        true.into()
    }

    let _ = unsafe { EnumWindows(Some(enum_proc), LPARAM(current_pid as usize as isize)) };
    let val = FOUND.load(Ordering::Relaxed);
    if val != 0 {
        Some(HWND(val as *mut _))
    } else {
        None
    }
}

#[allow(unsafe_code)]
#[cfg(windows)]
pub fn enable_transparency_opt_in() -> bool {
    use windows::Win32::Graphics::Dwm::{
        DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND,
    };
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, DeleteObject};

    let Some(hwnd) = find_main_hwnd() else {
        return false;
    };

    // Use a 1-pixel region outside the client area to opt DWM into
    // per-pixel alpha blending without applying any real blur.
    let rgn = unsafe { CreateRectRgn(0, 0, -1, -1) };
    let bb = DWM_BLURBEHIND {
        dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
        fEnable: true.into(),
        hRgnBlur: rgn,
        fTransitionOnMaximized: false.into(),
    };
    let res = unsafe { DwmEnableBlurBehindWindow(hwnd, &bb) };
    let _ = unsafe { DeleteObject(rgn.into()) };

    // Also apply rounded window clipping region (default 20pt)
    let _ = apply_rounded_window_corners(20.0);
    res.is_ok()
}

#[allow(unsafe_code)]
#[cfg(windows)]
pub fn apply_rounded_window_corners(radius_pt: f32) -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};
    use windows::Win32::UI::HiDpi::GetDpiForWindow;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let Some(hwnd) = find_main_hwnd() else {
        return false;
    };

    if radius_pt <= 0.0 {
        // Clear clipping region -> rectangular window
        unsafe { SetWindowRgn(hwnd, None, true) };
        return true;
    }

    let mut rect = RECT::default();
    if unsafe { GetClientRect(hwnd, &mut rect) }.is_err() {
        return false;
    }

    let w = rect.right - rect.left;
    let h = rect.bottom - rect.top;
    if w <= 0 || h <= 0 {
        return false;
    }

    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    let radius_px = (radius_pt * scale).round() as i32;
    let diam = radius_px * 2;

    let rgn = unsafe { CreateRoundRectRgn(0, 0, w + 1, h + 1, diam, diam) };
    if rgn.is_invalid() {
        return false;
    }

    // Windows takes ownership of `rgn` if SetWindowRgn succeeds.
    let ok = unsafe { SetWindowRgn(hwnd, Some(rgn), true) } != 0;
    if !ok {
        let _ = unsafe { DeleteObject(rgn.into()) };
        return false;
    }
    true
}

#[cfg(not(windows))]
pub fn enable_transparency_opt_in() -> bool {
    false
}

#[cfg(not(windows))]
pub fn apply_rounded_window_corners(_radius_pt: f32) -> bool {
    false
}
