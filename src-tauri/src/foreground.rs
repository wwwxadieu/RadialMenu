//! Tìm app dưới con trỏ: tên process, có chạy quyền Admin không, có đang toàn màn hình không.
//!
//! Các hàm ở đây có thể chờ app đích trả lời (WindowFromPoint gửi WM_NCHITTEST),
//! nên tuyệt đối không gọi trong callback của hook. Dùng qua `probe`.

use serde::Serialize;

/// Một app đang có cửa sổ mở, để chọn nhanh khi thêm profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppInfo {
    /// Tên file chương trình, chữ thường.
    pub process: String,
    /// Tiêu đề một cửa sổ của app đó.
    pub title: String,
}

/// Cửa sổ gốc dưới con trỏ và app sở hữu nó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// HWND của cửa sổ gốc, dạng số.
    pub hwnd: isize,
    pub pid: u32,
    /// Tên file chương trình, chữ thường: `chrome.exe`.
    pub process: String,
    /// Đường dẫn đầy đủ của file chương trình (để lấy icon).
    pub path: String,
    /// Lớp cửa sổ gốc, ví dụ `Progman` / `WorkerW` cho desktop, `CabinetWClass` cho Explorer.
    pub class: String,
    /// App chạy quyền cao hơn Radial Menu: Windows sẽ chặn phím gửi vào nó.
    pub elevated: bool,
    /// App đang active và chạy toàn màn hình (game, video, trình chiếu).
    pub fullscreen: bool,
}

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub fn at_point(x: i32, y: i32) -> Option<Target> {
    win::at_point(x, y)
}

#[cfg(not(windows))]
pub fn at_point(_x: i32, _y: i32) -> Option<Target> {
    None
}

/// Clipboard đang có dữ liệu (chữ, ảnh, file…). Không cần mở clipboard nên rất nhanh.
#[cfg(windows)]
pub fn clipboard_has_data() -> bool {
    unsafe { windows::Win32::System::DataExchange::CountClipboardFormats() > 0 }
}

#[cfg(not(windows))]
pub fn clipboard_has_data() -> bool {
    true
}

/// Các app đang có cửa sổ hiện trên taskbar, mỗi app một dòng (trừ chính Radial Menu).
#[cfg(windows)]
pub fn running_apps() -> Vec<AppInfo> {
    win::running_apps()
}

#[cfg(not(windows))]
pub fn running_apps() -> Vec<AppInfo> {
    Vec::new()
}
