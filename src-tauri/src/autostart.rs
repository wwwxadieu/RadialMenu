//! Chạy cùng Windows: một giá trị trong `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`
//! trỏ tới file exe đang chạy, kèm cờ `--autostart` để app khởi động ẩn dưới khay hệ thống.
//! Không cần quyền Admin.

#![cfg_attr(not(windows), allow(dead_code))]

/// Cờ dòng lệnh khi Windows tự mở app lúc đăng nhập.
pub const ARG: &str = "--autostart";
/// Tên giá trị trong khoá Run (hiện trong Task Manager › Startup apps).
const VALUE: &str = "Radial Menu";

/// App được Windows mở lúc đăng nhập (không phải người dùng tự mở).
pub fn launched_at_login() -> bool {
    std::env::args().skip(1).any(|a| a == ARG)
}

/// Lệnh ghi vào khoá Run: đường dẫn exe hiện tại trong ngoặc kép, kèm `--autostart`.
fn command_line() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Không tìm được đường dẫn app: {e}"))?;
    Ok(format!("\"{}\" {ARG}", exe.display()))
}

/// Bật hoặc tắt chạy cùng Windows.
pub fn set(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        if enabled {
            win::write(VALUE, &command_line()?)
        } else {
            win::remove(VALUE)
        }
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        Ok(())
    }
}

/// Lúc khởi động: đưa khoá Run về đúng cài đặt. Nếu app đã được cài sang chỗ khác
/// thì cập nhật lại đường dẫn. Bản debug không tự sửa để khỏi giành chỗ của bản đã cài.
pub fn sync(enabled: bool) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let current = win::read(VALUE);
        if enabled && current.as_deref() != Some(command_line()?.as_str()) {
            return set(true);
        }
        if !enabled && current.is_some() {
            return set(false);
        }
    }
    let _ = enabled;
    Ok(())
}

#[cfg(windows)]
mod win;
