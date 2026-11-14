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
    // Opt the transparent window into per-pixel alpha compositing.
    let res = unsafe { DwmEnableBlurBehindWindow(hwnd, &bb) };
    let _ = unsafe { DeleteObject(rgn.into()) };
    res.is_ok()
}

#[cfg(not(windows))]
pub fn enable_transparency_opt_in() -> bool {
    false
}
