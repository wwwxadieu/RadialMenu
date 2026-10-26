//! Phần Win32 của cửa sổ menu. Nhận HWND dạng con trỏ thô để không phụ thuộc
//! phiên bản crate `windows` mà Tauri đang dùng.

use core::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

/// Thêm WS_EX_NOACTIVATE (không bao giờ lấy focus) và WS_EX_TOOLWINDOW (không có
/// trong Alt+Tab). Gọi sau `set_ignore_cursor_events` để giữ các cờ nó đã đặt.
pub fn make_overlay(raw: *mut c_void) {
    let hwnd = HWND(raw);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let style = (style | WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style as isize);
        let _ = SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

/// Loại (hoặc thôi loại) cửa sổ menu khỏi mọi ảnh chụp màn hình (Windows 10 2004
/// trở lên), để có thể chụp nền phía sau ngay cả khi menu đang hiện.
/// Trả `false` nếu Windows quá cũ.
pub fn set_capture_exclusion(raw: *mut c_void, on: bool) -> bool {
    let affinity = if on { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE };
    unsafe { SetWindowDisplayAffinity(HWND(raw), affinity).is_ok() }
}

/// Đặt tâm cửa sổ tại (cx, cy), cạnh `size` px vật lý, rồi hiện mà không kích hoạt.
pub fn show_at(raw: *mut c_void, cx: i32, cy: i32, size: i32) {
    let hwnd = HWND(raw);
    let (left, top) = (cx - size / 2, cy - size / 2);
    unsafe {
        // Di chuyển trước: nếu sang màn hình khác DPI, Windows báo WM_DPICHANGED và
        // cửa sổ tự đổi kích thước. Lần gọi thứ hai đặt lại đúng vị trí và kích thước.
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            left,
            top,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            left,
            top,
            size,
            size,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

pub fn hide(raw: *mut c_void) {
    unsafe {
        let _ = ShowWindow(HWND(raw), SW_HIDE);
    }
}
