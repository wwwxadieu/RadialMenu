//! Cửa sổ menu tròn: tạo sẵn lúc khởi động, ẩn đi, chỉ di chuyển rồi hiện khi cần.
//!
//! Cửa sổ trong suốt, click xuyên qua và không bao giờ lấy focus, để phím tắt
//! gửi đi sau này vẫn vào đúng app đang dùng.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager, WebviewWindow};

pub const MENU_WINDOW: &str = "menu";
/// Cạnh cửa sổ menu theo px logic. Phải khớp `tauri.conf.json` và `src/menu/style.css`.
pub const MENU_SIZE: f64 = 520.0;
/// Đường kính đĩa kính theo px logic (khớp `.disc` trong CSS): vùng cần chụp làm nền mờ.
pub const DISC_SIZE: f64 = 344.0;
/// Thời gian chờ animation đóng chạy xong rồi mới ẩn cửa sổ.
pub const HIDE_DELAY: Duration = Duration::from_millis(360);
/// Kéo ra quá xa: giữa menu báo "Đóng" một nhịp rồi menu mới thu lại, nên chờ lâu hơn.
/// Phải dài hơn `AWAY_MS` trong `src/menu/main.ts`.
pub const AWAY_HIDE_DELAY: Duration = Duration::from_millis(540);

/// Tăng mỗi lần menu mở, để lệnh ẩn cũ không ẩn nhầm menu mới.
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Cửa sổ menu đã được loại khỏi ảnh chụp màn hình.
static EXCLUDED: AtomicBool = AtomicBool::new(false);

/// Có chụp nền được trong lúc menu đang hiện không (menu không lọt vào ảnh).
pub fn excluded_from_capture() -> bool {
    EXCLUDED.load(Ordering::Relaxed)
}

#[cfg(windows)]
mod win;

fn menu_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(MENU_WINDOW)
}

/// Chuẩn bị cửa sổ menu: click xuyên qua, không lấy focus, không hiện trong Alt+Tab.
/// `hide_from_capture`: loại menu khỏi ảnh chụp màn hình (cài đặt Giao diện).
pub fn setup(app: &AppHandle, hide_from_capture: bool) -> tauri::Result<()> {
    let Some(window) = menu_window(app) else {
        eprintln!("[radial] không tìm thấy cửa sổ menu");
        return Ok(());
    };
    window.set_ignore_cursor_events(true)?;
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        let raw = hwnd.0 as *mut core::ffi::c_void;
        win::make_overlay(raw);
        EXCLUDED.store(
            hide_from_capture && win::set_capture_exclusion(raw, true),
            Ordering::Relaxed,
        );
    }
    #[cfg(not(windows))]
    let _ = hide_from_capture;
    Ok(())
}

/// Bật/tắt việc loại menu khỏi ảnh chụp màn hình khi người dùng đổi cài đặt.
pub fn set_capture_exclusion(app: &AppHandle, on: bool) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = menu_window(&handle) else {
            return;
        };
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            let ok = win::set_capture_exclusion(hwnd.0 as *mut core::ffi::c_void, on);
            EXCLUDED.store(on && ok, Ordering::Relaxed);
        }
        #[cfg(not(windows))]
        let _ = (window, on);
    });
}

/// Đặt tâm menu tại (x, y) (toạ độ màn hình vật lý) rồi hiện ra, không lấy focus.
/// Trả về số thứ tự của lần mở này.
pub fn show(app: &AppHandle, x: i32, y: i32, scale: f64) -> u64 {
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    let size = (MENU_SIZE * scale).round() as i32;
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = menu_window(&handle) else {
            return;
        };
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            win::show_at(hwnd.0 as *mut core::ffi::c_void, x, y, size);
        }
        #[cfg(not(windows))]
        {
            let _ = window.set_position(tauri::PhysicalPosition::new(x - size / 2, y - size / 2));
            let _ = window.set_size(tauri::PhysicalSize::new(size as u32, size as u32));
            let _ = window.show();
        }
    });
    generation
}

/// Ẩn menu sau khi animation đóng chạy xong, trừ khi menu đã được mở lại.
pub fn hide_later(app: &AppHandle, delay: Duration) {
    let generation = GENERATION.load(Ordering::SeqCst);
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        if GENERATION.load(Ordering::SeqCst) != generation {
            return;
        }
        let inner = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            if GENERATION.load(Ordering::SeqCst) != generation {
                return;
            }
            let Some(window) = menu_window(&inner) else {
                return;
            };
            #[cfg(windows)]
            if let Ok(hwnd) = window.hwnd() {
                win::hide(hwnd.0 as *mut core::ffi::c_void);
            }
            #[cfg(not(windows))]
            let _ = window.hide();
        });
    });
}
