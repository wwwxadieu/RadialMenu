//! Chạy hành động của một góc menu, trên một thread riêng để không chặn app.

use std::process::Command;
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use crate::profiles::Action;

/// Nghỉ giữa các bước của một chuỗi để app kịp phản ứng.
const STEP_GAP: Duration = Duration::from_millis(40);
const MAX_WAIT_MS: u64 = 5_000;

#[derive(Debug, Clone)]
pub struct Job {
    pub label: String,
    pub action: Action,
    /// Click chuột trái tại `origin` trước khi chạy (chọn thứ dưới con trỏ).
    pub click_first: bool,
    /// Điểm nhấn chuột phải, toạ độ màn hình vật lý.
    pub origin: (i32, i32),
    /// Cửa sổ gốc dưới điểm nhấn: được kích hoạt trước khi gửi phím.
    pub target: Option<isize>,
}

pub struct Outcome {
    pub label: String,
    pub result: Result<(), String>,
}

pub struct Executor {
    tx: Sender<Job>,
}

impl Executor {
    pub fn start(report: impl Fn(Outcome) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("radial-actions".into())
            .spawn(move || {
                platform::init_thread();
                for job in rx {
                    let result = execute(&job);
                    report(Outcome {
                        label: job.label,
                        result,
                    });
                }
            })
            .expect("không tạo được thread radial-actions");
        Self { tx }
    }

    pub fn run(&self, job: Job) {
        let _ = self.tx.send(job);
    }
}

fn execute(job: &Job) -> Result<(), String> {
    if job.click_first {
        // Click cũng kích hoạt cửa sổ, nên không cần bước kích hoạt riêng.
        platform::click_at(job.origin.0, job.origin.1);
    } else if job.action.needs_focus() {
        if let Some(hwnd) = job.target {
            platform::activate(hwnd);
        }
    }
    perform(&job.action)
}

fn perform(action: &Action) -> Result<(), String> {
    match action {
        Action::Hotkey { keys } => {
            let parsed = crate::keys::parse_chord(keys)?;
            platform::send_keys(&parsed)
        }
        Action::Text { text } => platform::type_text(text),
        Action::Url { url } => platform::open(&expand_env(url)),
        Action::Command { cmd, args, cwd } => {
            let program = expand_env(cmd);
            let mut command = Command::new(&program);
            command.args(args.iter().map(|a| expand_env(a)));
            if let Some(dir) = cwd {
                command.current_dir(expand_env(dir));
            }
            command
                .spawn()
                .map(|_| ())
                .map_err(|e| format!("không chạy được {program}: {e}"))
        }
        Action::Sequence { steps } => {
            for (i, step) in steps.iter().enumerate() {
                if i > 0 {
                    std::thread::sleep(STEP_GAP);
                }
                perform(step)?;
            }
            Ok(())
        }
        // Mở cài đặt do lib.rs xử lý (cần AppHandle); tới được đây thì không làm gì.
        Action::Settings => Ok(()),
        Action::Wait { ms } => {
            std::thread::sleep(Duration::from_millis((*ms).min(MAX_WAIT_MS)));
            Ok(())
        }
    }
}

/// Giá trị của `%TÊN%`: biến môi trường, hoặc thư mục đặc biệt của Windows
/// (`%DESKTOP%`, `%DOCUMENTS%`, `%DOWNLOADS%`, đúng cả khi đã chuyển sang OneDrive).
fn lookup(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .or_else(|| platform::known_folder(name))
}

/// Đưa một cửa sổ lên trước, kể cả khi app khác đang active.
#[cfg(windows)]
pub fn activate_window(hwnd: isize) {
    platform::activate(hwnd);
}

/// Đường dẫn thật của `DESKTOP`, `DOCUMENTS` hoặc `DOWNLOADS`.
pub fn known_folder(name: &str) -> Option<String> {
    platform::known_folder(name)
}

/// Thay `%TÊN%` bằng giá trị của nó; tên không tồn tại thì giữ nguyên.
pub fn expand_env(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 => {
                let name = &after[..end];
                match lookup(name) {
                    Some(value) => out.push_str(&value),
                    None => {
                        out.push('%');
                        out.push_str(name);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(windows)]
mod win;
#[cfg(windows)]
use win as platform;

#[cfg(not(windows))]
mod platform {
    use crate::keys::Key;

    const UNSUPPORTED: &str = "chỉ chạy được trên Windows";

    pub fn init_thread() {}
    pub fn activate(_hwnd: isize) {}
    pub fn click_at(_x: i32, _y: i32) {}
    pub fn send_keys(_keys: &[Key]) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn type_text(_text: &str) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn open(_target: &str) -> Result<(), String> {
        Err(UNSUPPORTED.into())
    }
    pub fn known_folder(_name: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_expansion() {
        std::env::set_var("RADIAL_TEST_VAR", "abc");
        assert_eq!(expand_env("%RADIAL_TEST_VAR%\\x"), "abc\\x");
        assert_eq!(expand_env("100% chắc"), "100% chắc");
        assert_eq!(expand_env("%KHONG_CO_BIEN_NAY%"), "%KHONG_CO_BIEN_NAY%");
        assert_eq!(expand_env("a%%b"), "a%%b");
        assert_eq!(expand_env("không có gì"), "không có gì");
    }
}
