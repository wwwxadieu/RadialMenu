//! Hook chuột và bàn phím toàn hệ thống.
//!
//! Bản Windows nằm trong `hook/win.rs`. Trên nền tảng khác, `start` báo lỗi
//! để phần còn lại của app vẫn biên dịch và chạy được (giao diện, test).

use crate::gesture::{Config, Effect};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Mutex;

/// Dấu gắn vào sự kiện chuột/phím do chính app phát ra ("RADL").
#[cfg_attr(not(windows), allow(dead_code))]
pub const SIGNATURE: usize = 0x5241_444C;

/// Hook gọi hàm này lúc nhấn chuột phải tại (x, y); trả `true` = để Windows xử lý.
/// Hàm phải trả lời nhanh (vài ms), vì nó chạy trong callback của hook.
pub type BypassFn = Box<dyn Fn(i32, i32) -> bool + Send + 'static>;

static PAUSED: AtomicBool = AtomicBool::new(false);
/// Cấu hình cử chỉ mới, hook lấy vào ở lần nhấn kế tiếp.
static PENDING_CONFIG: Mutex<Option<Config>> = Mutex::new(None);

/// Tạm dừng: mọi cú nhấn chuột phải đi thẳng tới Windows.
pub fn set_paused(paused: bool) {
    PAUSED.store(paused, Ordering::Relaxed);
}

pub fn is_paused() -> bool {
    PAUSED.load(Ordering::Relaxed)
}

/// Đổi ngưỡng kéo / vùng huỷ khi đang chạy (sau khi nạp lại cài đặt).
pub fn set_config(cfg: Config) {
    *PENDING_CONFIG.lock().unwrap() = Some(cfg);
}

#[cfg_attr(not(windows), allow(dead_code))]
fn take_pending_config() -> Option<Config> {
    PENDING_CONFIG.lock().ok()?.take()
}

#[cfg(windows)]
mod win;

#[cfg(windows)]
pub use win::HookHandle;

/// Cài hook trên một thread riêng. Mọi hiệu ứng cử chỉ (khác `Effect::None`)
/// được gửi vào `events`.
#[cfg(windows)]
pub fn start(cfg: Config, events: Sender<Effect>, bypass: BypassFn) -> std::io::Result<HookHandle> {
    win::start(cfg, events, bypass)
}

#[cfg(not(windows))]
pub struct HookHandle;

#[cfg(not(windows))]
impl HookHandle {
    pub fn stop(&mut self) {}
}

#[cfg(not(windows))]
pub fn start(
    _cfg: Config,
    _events: Sender<Effect>,
    _bypass: BypassFn,
) -> std::io::Result<HookHandle> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Hook chuột chỉ có trên Windows",
    ))
}
