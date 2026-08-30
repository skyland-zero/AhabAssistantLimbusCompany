//! Small Windows shell adapter kept independent from the GPUI view tree.
//!
//! GPUI owns the actual window and event loop. This module only supplies the
//! process-wide mutex and a native notification-area menu so the Python
//! sidecar never has to know about desktop shell concerns.

#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::upper_case_acronyms)]

use std::{ffi::c_void, mem, ptr, thread};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

type HINSTANCE = *mut c_void;
type HANDLE = *mut c_void;
type HWND = *mut c_void;
type HICON = *mut c_void;
type HBRUSH = *mut c_void;
type HCURSOR = *mut c_void;
type HMENU = *mut c_void;
type LPCWSTR = *const u16;
type LPVOID = *mut c_void;
type WPARAM = usize;
type LPARAM = isize;
type LRESULT = isize;
type UINT = u32;
type DWORD = u32;
type BOOL = i32;

const ERROR_ALREADY_EXISTS: DWORD = 183;
const WM_DESTROY: UINT = 0x0002;
const WM_COMMAND: UINT = 0x0111;
const WM_CONTEXTMENU: UINT = 0x007b;
const WM_APP: UINT = 0x8000;
const WM_LBUTTONDBLCLK: UINT = 0x0203;
const WM_RBUTTONUP: UINT = 0x0205;
const WM_CLOSE: UINT = 0x0010;
const SW_HIDE: i32 = 0;
const SW_SHOWNORMAL: i32 = 1;
const NIF_MESSAGE: UINT = 0x0001;
const NIF_ICON: UINT = 0x0002;
const NIF_TIP: UINT = 0x0004;
const NIM_ADD: DWORD = 0;
const NIM_DELETE: DWORD = 2;
const TPM_LEFTALIGN: UINT = 0;
const TPM_BOTTOMALIGN: UINT = 0x20;
const TPM_RIGHTBUTTON: UINT = 0x0002;
const MF_STRING: UINT = 0;
const ID_SHOW: usize = 1001;
const ID_HIDE: usize = 1002;
const ID_EXIT: usize = 1003;
const TRAY_MESSAGE: UINT = WM_APP + 1;

#[repr(C)]
struct WndClassW {
    style: UINT,
    lpfn_wnd_proc: Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: HINSTANCE,
    h_icon: HICON,
    h_cursor: HCURSOR,
    hbr_background: HBRUSH,
    lpsz_menu_name: LPCWSTR,
    lpsz_class_name: LPCWSTR,
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
    time: DWORD,
    point: Point,
    private: DWORD,
}

#[repr(C)]
struct NotifyIconDataW {
    cb_size: DWORD,
    h_wnd: HWND,
    u_id: UINT,
    u_flags: UINT,
    u_callback_message: UINT,
    h_icon: HICON,
    sz_tip: [u16; 128],
    dw_state: DWORD,
    dw_state_mask: DWORD,
    sz_info: [u16; 256],
    anonymous: [u16; 64],
    dw_info_flags: DWORD,
    guid_item: [u8; 16],
    h_balloon_icon: HICON,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attributes: LPVOID, initial_owner: BOOL, name: LPCWSTR) -> HANDLE;
    fn GetLastError() -> DWORD;
    fn CloseHandle(object: HANDLE) -> BOOL;
    fn GetModuleHandleW(module_name: LPCWSTR) -> HINSTANCE;
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(window_class: *const WndClassW) -> u16;
    fn CreateWindowExW(
        ex_style: DWORD,
        class_name: LPCWSTR,
        window_name: LPCWSTR,
        style: DWORD,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: HMENU,
        instance: HINSTANCE,
        param: LPVOID,
    ) -> HWND;
    fn DestroyWindow(window: HWND) -> BOOL;
    fn DefWindowProcW(window: HWND, message: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn FindWindowW(class_name: LPCWSTR, window_name: LPCWSTR) -> HWND;
    fn SetForegroundWindow(window: HWND) -> BOOL;
    fn ShowWindow(window: HWND, command: i32) -> BOOL;
    fn PostMessageW(window: HWND, message: UINT, w_param: WPARAM, l_param: LPARAM) -> BOOL;
    fn GetCursorPos(point: *mut Point) -> BOOL;
    fn CreatePopupMenu() -> HMENU;
    fn AppendMenuW(menu: HMENU, flags: UINT, identifier: usize, text: LPCWSTR) -> BOOL;
    fn TrackPopupMenu(
        menu: HMENU,
        flags: UINT,
        x: i32,
        y: i32,
        reserved: i32,
        owner: HWND,
        rect: LPVOID,
    ) -> BOOL;
    fn DestroyMenu(menu: HMENU) -> BOOL;
    fn LoadIconW(instance: HINSTANCE, icon_name: LPCWSTR) -> HICON;
    fn GetMessageW(message: *mut Msg, window: HWND, min_filter: UINT, max_filter: UINT) -> i32;
    fn TranslateMessage(message: *const Msg) -> BOOL;
    fn DispatchMessageW(message: *const Msg) -> LRESULT;
    fn PostQuitMessage(exit_code: i32);
    fn IsIconic(window: HWND) -> BOOL;
}

#[link(name = "shell32")]
unsafe extern "system" {
    fn Shell_NotifyIconW(message: DWORD, data: *const NotifyIconDataW) -> BOOL;
}

pub struct InstanceGuard {
    handle: HANDLE,
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

pub fn acquire_instance() -> Result<Option<InstanceGuard>, String> {
    let name = wide("Global\\AALC.GPUI.SingleInstance");
    let handle = unsafe { CreateMutexW(ptr::null_mut(), 0, name.as_ptr()) };
    if handle.is_null() {
        return Err(format!("无法创建 AALC 单实例互斥体：{}", unsafe {
            GetLastError()
        }));
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = CloseHandle(handle);
        }
        focus_existing_window();
        return Ok(None);
    }
    Ok(Some(InstanceGuard { handle }))
}

pub fn start_tray() {
    thread::Builder::new()
        .name("AhabWindowsTray".to_owned())
        .spawn(|| unsafe { tray_thread() })
        .expect("启动 Windows 托盘线程失败");
}

pub fn is_window_minimized(window: &gpui::Window) -> bool {
    let Ok(handle) = <gpui::Window as HasWindowHandle>::window_handle(window) else {
        return false;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return false;
    };
    unsafe { IsIconic(handle.hwnd.get() as HWND) != 0 }
}

fn focus_existing_window() {
    let title = wide("AALC · GPUI");
    let window = unsafe { FindWindowW(ptr::null(), title.as_ptr()) };
    if !window.is_null() {
        unsafe {
            let _ = ShowWindow(window, SW_SHOWNORMAL);
            let _ = SetForegroundWindow(window);
        }
    }
}

unsafe fn tray_thread() {
    let instance = GetModuleHandleW(ptr::null());
    let class_name = wide("AALC.GPUI.Tray");
    let window_class = WndClassW {
        style: 0,
        lpfn_wnd_proc: Some(tray_window_proc),
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: instance,
        h_icon: LoadIconW(instance, 1_usize as LPCWSTR),
        h_cursor: ptr::null_mut(),
        hbr_background: ptr::null_mut(),
        lpsz_menu_name: ptr::null(),
        lpsz_class_name: class_name.as_ptr(),
    };
    if RegisterClassW(&window_class) == 0 {
        return;
    }
    let window = CreateWindowExW(
        0,
        class_name.as_ptr(),
        class_name.as_ptr(),
        0,
        0,
        0,
        0,
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        instance,
        ptr::null_mut(),
    );
    if window.is_null() {
        return;
    }

    let mut icon_data = NotifyIconDataW {
        cb_size: mem::size_of::<NotifyIconDataW>() as DWORD,
        h_wnd: window,
        u_id: 1,
        u_flags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        u_callback_message: TRAY_MESSAGE,
        h_icon: LoadIconW(instance, 1_usize as LPCWSTR),
        sz_tip: [0; 128],
        dw_state: 0,
        dw_state_mask: 0,
        sz_info: [0; 256],
        anonymous: [0; 64],
        dw_info_flags: 0,
        guid_item: [0; 16],
        h_balloon_icon: ptr::null_mut(),
    };
    copy_wide("AALC · GPUI", &mut icon_data.sz_tip);
    if Shell_NotifyIconW(NIM_ADD, &icon_data) == 0 {
        let _ = DestroyWindow(window);
        return;
    }

    let mut message = Msg {
        hwnd: ptr::null_mut(),
        message: 0,
        w_param: 0,
        l_param: 0,
        time: 0,
        point: Point { x: 0, y: 0 },
        private: 0,
    };
    while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
        let _ = TranslateMessage(&message);
        let _ = DispatchMessageW(&message);
    }
    let _ = Shell_NotifyIconW(NIM_DELETE, &icon_data);
    let _ = DestroyWindow(window);
}

unsafe extern "system" fn tray_window_proc(
    window: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match message {
        TRAY_MESSAGE if l_param as UINT == WM_LBUTTONDBLCLK => {
            show_main_window();
            0
        }
        TRAY_MESSAGE if l_param as UINT == WM_RBUTTONUP || l_param as UINT == WM_CONTEXTMENU => {
            show_tray_menu(window);
            0
        }
        WM_COMMAND => match w_param & 0xffff {
            ID_SHOW => {
                show_main_window();
                0
            }
            ID_HIDE => {
                hide_main_window();
                0
            }
            ID_EXIT => {
                let title = wide("AALC · GPUI");
                let main_window = FindWindowW(ptr::null(), title.as_ptr());
                if !main_window.is_null() {
                    let _ = PostMessageW(main_window, WM_CLOSE, 0, 0);
                }
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(window, message, w_param, l_param),
        },
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(window, message, w_param, l_param),
    }
}

unsafe fn show_tray_menu(owner: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let show = wide("显示 AALC");
    let hide = wide("隐藏 AALC");
    let exit = wide("退出");
    let _ = AppendMenuW(menu, MF_STRING, ID_SHOW, show.as_ptr());
    let _ = AppendMenuW(menu, MF_STRING, ID_HIDE, hide.as_ptr());
    let _ = AppendMenuW(menu, MF_STRING, ID_EXIT, exit.as_ptr());
    let mut point = Point { x: 0, y: 0 };
    let _ = GetCursorPos(&mut point);
    let _ = SetForegroundWindow(owner);
    let _ = TrackPopupMenu(
        menu,
        TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RIGHTBUTTON,
        point.x,
        point.y,
        0,
        owner,
        ptr::null_mut(),
    );
    let _ = DestroyMenu(menu);
}

unsafe fn show_main_window() {
    let title = wide("AALC · GPUI");
    let window = FindWindowW(ptr::null(), title.as_ptr());
    if !window.is_null() {
        let _ = ShowWindow(window, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(window);
    }
}

unsafe fn hide_main_window() {
    let title = wide("AALC · GPUI");
    let window = FindWindowW(ptr::null(), title.as_ptr());
    if !window.is_null() {
        let _ = ShowWindow(window, SW_HIDE);
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn copy_wide(value: &str, target: &mut [u16]) {
    for (slot, character) in target.iter_mut().zip(value.encode_utf16()) {
        *slot = character;
    }
}
