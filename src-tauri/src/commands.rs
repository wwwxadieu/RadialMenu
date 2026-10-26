//! Lệnh cho cửa sổ cài đặt và trang menu (gọi qua `invoke` từ giao diện).
//!
//! Mọi lệnh dùng `try_state`: cửa sổ có thể hỏi trước khi `setup` nạp xong cấu hình.

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::foreground::AppInfo;
use crate::profiles::{self, Profile, Store};
use crate::settings::{Settings, ACCENTS};
use crate::{autostart, backup, hook, AppState, LogEvent, MenuLook};

const STARTING: &str = "Radial Menu đang khởi động, thử lại sau giây lát";

fn state(app: &AppHandle) -> Result<tauri::State<'_, AppState>, String> {
    app.try_state::<AppState>()
        .ok_or_else(|| STARTING.to_string())
}

/// Mọi thứ cửa sổ cài đặt cần để vẽ.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatePayload {
    /// Cấu hình chưa nạp xong: các trường còn lại là giá trị mặc định.
    starting: bool,
    paused: bool,
    hooked: bool,
    hook_error: Option<String>,
    settings: Settings,
    /// Cài đặt gốc, cho nút "Đặt lại".
    defaults: Settings,
    profiles: Vec<Profile>,
    /// Bản gốc của các profile có sẵn, để đặt lại từng profile.
    builtins: Vec<Profile>,
    accents: [&'static str; 5],
    config_dir: String,
    load_errors: Vec<String>,
    /// App gần đây dưới con trỏ khi mở menu, mới nhất trước.
    recent: Vec<String>,
    version: &'static str,
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> StatePayload {
    let mut payload = StatePayload {
        starting: true,
        paused: hook::is_paused(),
        hooked: false,
        hook_error: None,
        settings: Settings::default(),
        defaults: Settings::default(),
        profiles: Vec::new(),
        builtins: profiles::defaults(),
        accents: ACCENTS,
        config_dir: String::new(),
        load_errors: Vec::new(),
        recent: Vec::new(),
        version: env!("CARGO_PKG_VERSION"),
    };
    let Some(state) = app.try_state::<AppState>() else {
        return payload;
    };

    payload.hooked = state.hook.lock().unwrap().is_some();
    payload.hook_error = state.hook_error.lock().unwrap().clone();
    payload.starting = !payload.hooked && payload.hook_error.is_none();
    payload.settings = state.settings.read().unwrap().clone();
    payload.profiles = state.profiles.read().unwrap().profiles.clone();
    payload.config_dir = state.config_dir.display().to_string();
    payload.load_errors = state.load_errors.lock().unwrap().clone();
    payload.recent = state.recent.lock().unwrap().clone();
    payload
}

/// Lưu một profile. `original_id` = mã trước khi sửa, `None` khi thêm mới.
#[tauri::command]
pub fn save_profile(
    app: AppHandle,
    profile: Profile,
    original_id: Option<String>,
) -> Result<Profile, String> {
    let state = state(&app)?;
    let saved = {
        let store = state.profiles.read().unwrap();
        profiles::save_profile(
            &state.profiles_dir(),
            &store,
            profile,
            original_id.as_deref(),
        )?
    };
    crate::reload(&app);
    Ok(saved)
}

#[tauri::command]
pub fn delete_profile(app: AppHandle, id: String) -> Result<(), String> {
    let state = state(&app)?;
    {
        let store = state.profiles.read().unwrap();
        profiles::delete_profile(&state.profiles_dir(), &store, &id)?;
    }
    crate::reload(&app);
    Ok(())
}

/// Lưu và áp dụng ngay cài đặt chung. Trả về bản đã chuẩn hoá.
#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings, String> {
    let state = state(&app)?;
    let current = state.settings.read().unwrap().clone();
    let mut next = settings.sanitized();
    next.profiles_version = current.profiles_version;
    next.settings_version = current.settings_version;

    if next.launch_at_login != current.launch_at_login {
        autostart::set(next.launch_at_login)?;
    }
    next.save(&state.settings_path())
        .map_err(|e| format!("Không ghi được settings.json: {e}"))?;

    crate::apply_settings(&app, &next);
    *state.settings.write().unwrap() = next.clone();
    crate::config_changed(&app);
    Ok(next)
}

/// Ghi lại các profile có sẵn về bản gốc. Profile tự tạo được giữ nguyên.
#[tauri::command]
pub fn restore_defaults(app: AppHandle) -> Result<usize, String> {
    let state = state(&app)?;
    profiles::write_defaults(&state.profiles_dir())
        .map_err(|e| format!("Không ghi được profile mặc định: {e}"))?;
    crate::reload(&app);
    Ok(profiles::defaults().len())
}

/// Ghi bản sao lưu vào thư mục Downloads (không có thì vào thư mục cấu hình),
/// mở Explorer chọn sẵn file đó. Trả về đường dẫn file.
#[tauri::command]
pub fn export_backup(app: AppHandle) -> Result<String, String> {
    let state = state(&app)?;
    let text = {
        let settings = state.settings.read().unwrap();
        let store = state.profiles.read().unwrap();
        backup::to_json(&settings, &store.profiles)
    };
    let dir = crate::actions::known_folder("DOWNLOADS")
        .map(std::path::PathBuf::from)
        .filter(|d| d.is_dir())
        .unwrap_or_else(|| state.config_dir.clone());
    let path = dir.join(backup::file_name());
    std::fs::write(&path, text).map_err(|e| format!("Không ghi được {}: {e}", path.display()))?;
    reveal(&path);
    Ok(path.display().to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    profiles: Vec<String>,
    settings: bool,
}

/// Nhập bản sao lưu hoặc một file profile. Profile trùng mã sẽ bị ghi đè.
/// Riêng "chạy cùng Windows" giữ theo máy hiện tại.
#[tauri::command]
pub fn import_backup(app: AppHandle, text: String) -> Result<ImportSummary, String> {
    let state = state(&app)?;
    let parsed = backup::parse(&text)?;
    let dir = state.profiles_dir();

    let mut names = Vec::new();
    for profile in parsed.profiles {
        // Nạp lại sau mỗi lần ghi để biết profile nào đã có (và file cũ của nó).
        let (store, _) = Store::load_dir(&dir);
        let existing = store
            .profiles
            .iter()
            .any(|p| p.id == profile.id)
            .then(|| profile.id.clone());
        let saved = profiles::save_profile(&dir, &store, profile, existing.as_deref())?;
        names.push(saved.name);
    }

    let has_settings = parsed.settings.is_some();
    if let Some(mut settings) = parsed.settings {
        let current = state.settings.read().unwrap().clone();
        settings.profiles_version = current.profiles_version;
        settings.settings_version = current.settings_version;
        settings.launch_at_login = current.launch_at_login;
        settings
            .save(&state.settings_path())
            .map_err(|e| format!("Không ghi được settings.json: {e}"))?;
    }
    crate::reload(&app);
    Ok(ImportSummary {
        profiles: names,
        settings: has_settings,
    })
}

/// Các app đang mở cửa sổ, để chọn khi thêm profile.
#[tauri::command]
pub fn list_apps() -> Vec<AppInfo> {
    let mut apps = crate::foreground::running_apps();
    apps.sort_by(|a, b| a.process.cmp(&b.process));
    apps
}

#[tauri::command]
pub fn set_paused(app: AppHandle, paused: bool) -> bool {
    crate::apply_paused(&app, paused);
    paused
}

#[tauri::command]
pub fn reload_config(app: AppHandle) {
    crate::reload(&app);
}

#[tauri::command]
pub fn open_config_dir(app: AppHandle) {
    crate::open_config_folder(&app);
}

/// Trang menu hỏi tông màu, màu nhấn, animation khi vừa tải xong.
#[tauri::command]
pub fn menu_look(app: AppHandle) -> MenuLook {
    match app.try_state::<AppState>() {
        Some(state) => MenuLook::from(&state.settings.read().unwrap()),
        None => MenuLook::from(&Settings::default()),
    }
}

/// Trang menu báo đã vẽ khung hình đầu tiên của lần mở `seq`.
#[tauri::command]
pub fn menu_frame(app: AppHandle, seq: u64) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let started = {
        let mut pending = state.pending_frame.lock().unwrap();
        match *pending {
            Some((s, t0)) if s == seq => {
                *pending = None;
                t0
            }
            _ => return,
        }
    };
    crate::log(
        &app,
        LogEvent::Latency {
            ms: started.elapsed().as_secs_f64() * 1000.0,
        },
    );
}

/// Mở Explorer và chọn sẵn file.
fn reveal(path: &std::path::Path) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("explorer")
            .raw_arg(format!("/select,\"{}\"", path.display()))
            .spawn();
    }
    #[cfg(not(windows))]
    let _ = path;
}
