//! Hỏi "app dưới con trỏ là gì" mà không làm treo hook.
//!
//! Hook gọi `check` ngay lúc nhấn chuột phải. Việc tra cứu chạy trên thread riêng;
//! hook chỉ chờ tối đa `HOOK_WAIT`. App đích đang bận thì coi như không bỏ qua,
//! thay vì để Windows gỡ hook vì callback chạy quá lâu.

use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::foreground::{self, Target};
use crate::settings::Settings;

const HOOK_WAIT: Duration = Duration::from_millis(25);
const STALE: Duration = Duration::from_millis(80);

struct Request {
    x: i32,
    y: i32,
    at: Instant,
    reply: SyncSender<bool>,
}

/// Lý do bỏ qua menu tròn, để ghi vào nhật ký.
pub fn bypass_reason(target: &Target, settings: &Settings) -> Option<&'static str> {
    if target.elevated {
        Some("app chạy quyền Admin")
    } else if settings.is_excluded(&target.process) {
        Some("app trong danh sách loại trừ")
    } else if settings.disable_when_fullscreen && target.fullscreen {
        Some("app đang chạy toàn màn hình")
    } else {
        None
    }
}

/// Kết quả tra cứu gần nhất: điểm nhấn và app tại đó.
type LastProbe = Option<((i32, i32), Option<Target>)>;

pub struct Probe {
    tx: Mutex<Sender<Request>>,
    last: Arc<Mutex<LastProbe>>,
}

impl Probe {
    /// `on_bypass` được gọi (trên thread của probe) mỗi khi một cú nhấn bị bỏ qua.
    pub fn spawn(
        settings: Arc<RwLock<Settings>>,
        on_bypass: impl Fn(&Target, &'static str) + Send + 'static,
    ) -> Arc<Self> {
        let (tx, rx): (Sender<Request>, Receiver<Request>) = mpsc::channel();
        let last = Arc::new(Mutex::new(None));
        let shared = last.clone();
        std::thread::Builder::new()
            .name("radial-probe".into())
            .spawn(move || {
                for req in rx {
                    if req.at.elapsed() > STALE {
                        continue; // hook đã thôi chờ từ lâu
                    }
                    let target = foreground::at_point(req.x, req.y);
                    let reason = target
                        .as_ref()
                        .and_then(|t| bypass_reason(t, &settings.read().unwrap()));
                    let _ = req.reply.send(reason.is_some());
                    if let (Some(t), Some(r)) = (&target, reason) {
                        on_bypass(t, r);
                    }
                    *shared.lock().unwrap() = Some(((req.x, req.y), target));
                }
            })
            .expect("không tạo được thread radial-probe");
        Arc::new(Self {
            tx: Mutex::new(tx),
            last,
        })
    }

    /// Gọi từ hook: có nên để Windows xử lý cú nhấn tại (x, y) không.
    pub fn check(&self, x: i32, y: i32) -> bool {
        let (reply, answer) = mpsc::sync_channel(1);
        let request = Request {
            x,
            y,
            at: Instant::now(),
            reply,
        };
        if self.tx.lock().unwrap().send(request).is_err() {
            return false;
        }
        answer.recv_timeout(HOOK_WAIT).unwrap_or(false)
    }

    /// App tại điểm nhấn (x, y): dùng lại kết quả lúc nhấn nếu có, không thì tra lại.
    pub fn target_at(&self, x: i32, y: i32) -> Option<Target> {
        if let Some(((lx, ly), target)) = self.last.lock().unwrap().clone() {
            if (lx, ly) == (x, y) {
                return target;
            }
        }
        foreground::at_point(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(process: &str) -> Target {
        Target {
            hwnd: 1,
            pid: 1,
            process: process.into(),
            path: String::new(),
            class: String::new(),
            elevated: false,
            fullscreen: false,
        }
    }

    #[test]
    fn reasons() {
        let s = Settings::default();
        assert_eq!(bypass_reason(&target("chrome.exe"), &s), None);
        assert_eq!(
            bypass_reason(&target("blender.exe"), &s),
            Some("app trong danh sách loại trừ")
        );
        let admin = Target {
            elevated: true,
            ..target("cmd.exe")
        };
        assert_eq!(bypass_reason(&admin, &s), Some("app chạy quyền Admin"));
        let game = Target {
            fullscreen: true,
            ..target("game.exe")
        };
        assert_eq!(
            bypass_reason(&game, &s),
            Some("app đang chạy toàn màn hình")
        );
        let s2 = Settings {
            disable_when_fullscreen: false,
            ..Settings::default()
        };
        assert_eq!(bypass_reason(&game, &s2), None);
    }
}
