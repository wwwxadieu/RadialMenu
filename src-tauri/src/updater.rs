//! Tự cập nhật: hỏi GitHub Releases (`latest.json`) có bản mới không, rồi tải và cài
//! khi người dùng đồng ý. Không bao giờ tự cài khi chưa hỏi.
//!
//! Bản cài được ký bằng khoá updater (xem `.github/workflows/release.yml`); khoá công
//! khai nằm trong `tauri.conf.json`, nên file tải về bị sửa sẽ không được cài.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::{AppState, MAIN_WINDOW};

/// Chờ một lúc sau khi khởi động rồi mới hỏi, để không tranh mạng lúc đăng nhập.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
/// Hỏi lại mỗi ngày nếu app chạy liên tục.
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Bản mới tìm thấy, gửi sang cửa sổ cài đặt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    pub notes: String,
    /// Ngày phát hành, dạng YYYY-MM-DD.
    pub date: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

#[derive(Default)]
pub struct UpdateState {
    /// Bản mới lần hỏi gần nhất (để cài mà không phải hỏi lại).
    pending: Mutex<Option<Update>>,
    /// Đang tải hoặc cài: chặn bấm cập nhật hai lần.
    busy: Mutex<bool>,
}

impl UpdateState {
    pub fn info(&self) -> Option<UpdateInfo> {
        self.pending.lock().unwrap().as_ref().map(info_of)
    }
}

fn info_of(u: &Update) -> UpdateInfo {
    UpdateInfo {
        version: u.version.clone(),
        current: u.current_version.clone(),
        notes: u.body.clone().unwrap_or_default().trim().to_string(),
        date: u
            .date
            .map(|d| format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())),
    }
}

/// Lỗi của updater, viết lại cho người dùng đọc được.
fn friendly(e: tauri_plugin_updater::Error) -> String {
    let text = e.to_string();
    let lower = text.to_lowercase();
    if lower.contains("signature") {
        "Bản tải về không có chữ ký hợp lệ nên không được cài.".into()
    } else if lower.contains("dns")
        || lower.contains("connect")
        || lower.contains("timed out")
        || lower.contains("network")
    {
        "Không kết nối được tới GitHub. Kiểm tra mạng rồi thử lại.".into()
    } else if lower.contains("404") || lower.contains("not found") {
        "Chưa có bản phát hành nào có thông tin cập nhật.".into()
    } else {
        format!("Không kiểm tra được bản mới: {text}")
    }
}

/// Hỏi có bản mới không. Có thì nhớ lại, báo ra khay và cửa sổ cài đặt.
pub async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater().map_err(friendly)?;
    let found = updater.check().await.map_err(friendly)?;
    let state = app.state::<UpdateState>();
    let info = found.as_ref().map(info_of);
    *state.pending.lock().unwrap() = found;
    match &info {
        Some(i) => {
            crate::tray::show_update(app, &i.version);
            let _ = app.emit_to(MAIN_WINDOW, "update:available", i.clone());
        }
        None => crate::tray::hide_update(app),
    }
    Ok(info)
}

/// Tải bản mới đã tìm thấy rồi chạy trình cài đặt. Trên Windows app tự thoát khi
/// trình cài đặt chạy, và được mở lại sau khi cài xong.
pub async fn install(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<UpdateState>();
    {
        let mut busy = state.busy.lock().unwrap();
        if *busy {
            return Err("Đang cập nhật, chờ một chút.".into());
        }
        *busy = true;
    }
    let result = install_inner(app).await;
    *state.busy.lock().unwrap() = false;
    result
}

async fn install_inner(app: &AppHandle) -> Result<(), String> {
    let pending = app.state::<UpdateState>().pending.lock().unwrap().clone();
    let update = match pending {
        Some(u) => u,
        // Chưa hỏi (hoặc app vừa khởi động): hỏi lại một lần.
        None => app
            .updater()
            .map_err(friendly)?
            .check()
            .await
            .map_err(friendly)?
            .ok_or_else(|| "Bạn đang dùng bản mới nhất.".to_string())?,
    };

    let mut downloaded: u64 = 0;
    let progress_app = app.clone();
    let done_app = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = progress_app.emit_to(
                    MAIN_WINDOW,
                    "update:progress",
                    Progress { downloaded, total },
                );
            },
            move || {
                let _ = done_app.emit_to(MAIN_WINDOW, "update:installing", ());
            },
        )
        .await
        .map_err(friendly)
}

/// Tự hỏi bản mới sau khi khởi động và mỗi ngày một lần (nếu cài đặt cho phép).
/// Chạy trên một thread riêng: chỉ ngủ rồi gọi `check`, không giữ luồng async nào.
pub fn spawn_auto_check(app: AppHandle) {
    let _ = std::thread::Builder::new()
        .name("radial-update".into())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK_DELAY);
            loop {
                let enabled = app
                    .try_state::<AppState>()
                    .map(|s| s.settings.read().unwrap().check_updates)
                    .unwrap_or(false);
                if enabled {
                    if let Err(e) = tauri::async_runtime::block_on(check(&app)) {
                        eprintln!("[radial] {e}");
                    }
                }
                std::thread::sleep(CHECK_INTERVAL);
            }
        });
}
