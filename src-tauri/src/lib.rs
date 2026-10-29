mod actions;
mod appicon;
mod autostart;
mod backdrop;
mod backup;
mod commands;
mod foreground;
mod gesture;
mod hook;
mod keys;
mod overlay;
mod probe;
mod profiles;
mod settings;
mod tray;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, RunEvent, Theme, WindowEvent};

use actions::{Executor, Job};
use foreground::Target;
use gesture::{Dir, Effect, DIR_NAMES};
use probe::Probe;
use profiles::{Context, Profile, Store};
use settings::Settings;

/// Nhãn cửa sổ cài đặt.
pub const MAIN_WINDOW: &str = "main";
/// Số app gần đây (dưới con trỏ khi mở menu) nhớ lại để gợi ý khi thêm profile.
const RECENT_LIMIT: usize = 12;

/// Một lần mở menu: app dưới con trỏ và profile đang hiện.
struct Session {
    origin: (i32, i32),
    target: Option<Target>,
    profile: Profile,
    /// Tình trạng máy lúc mở (clipboard…), quyết định góc nào dùng được.
    ctx: Context,
}

struct AppState {
    config_dir: PathBuf,
    settings: Arc<RwLock<Settings>>,
    profiles: RwLock<Store>,
    load_errors: Mutex<Vec<String>>,
    hook: Mutex<Option<hook::HookHandle>>,
    hook_error: Mutex<Option<String>>,
    probe: OnceLock<Arc<Probe>>,
    executor: OnceLock<Executor>,
    session: Mutex<Option<Session>>,
    /// Hàng đợi chụp nền mờ (chạy ngoài thread sự kiện để không chặn menu).
    backdrop: OnceLock<std::sync::mpsc::Sender<BackdropJob>>,
    /// Lần mở menu đang chờ đo độ trễ: (số thứ tự, lúc nhận cử chỉ mở).
    pending_frame: Mutex<Option<(u64, Instant)>>,
    /// Tên chương trình gần đây nhất ở dưới con trỏ khi mở menu, mới nhất trước.
    recent: Mutex<Vec<String>>,
    /// Icon thật của các app đã gặp.
    icons: Arc<appicon::IconCache>,
    /// Hàng đợi đọc icon app chưa có trong bộ đệm.
    icon_jobs: OnceLock<std::sync::mpsc::Sender<IconJob>>,
}

impl AppState {
    fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    fn profiles_dir(&self) -> PathBuf {
        self.config_dir.join("profiles")
    }
}

/// Sự kiện gửi sang trang Nhật ký của cửa sổ cài đặt qua kênh `gesture`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum LogEvent {
    /// Kéo vượt ngưỡng: menu mở tại (x, y) với profile của app dưới con trỏ.
    Open {
        x: i32,
        y: i32,
        scale: f64,
        dir: Option<Dir>,
        process: Option<String>,
        profile: String,
    },
    /// Hướng đang chỉ đổi. `None` = vùng huỷ ở giữa.
    Dir { dir: Option<Dir> },
    /// Nhả ở giữa, bấm Esc hoặc nút khác: không chạy gì.
    Cancel,
    /// Nhả ở một góc không có lệnh.
    Empty { dir: Dir },
    /// Kéo ra quá xa: menu tự đóng, không chạy gì.
    Away,
    /// Nhả ở một góc đang tắt (ví dụ Dán khi clipboard trống).
    Unavailable {
        dir: Dir,
        label: String,
        reason: String,
    },
    /// Đã gửi lệnh của một góc.
    Run {
        dir: Dir,
        label: String,
        detail: String,
        process: Option<String>,
    },
    /// Lệnh chạy lỗi.
    Error { label: String, message: String },
    /// Nhả trước ngưỡng: đã phát lại right-click để mở menu gốc.
    Native,
    /// Có nút khác được nhấn khi đang giữ: trả nút phải lại cho Windows.
    Handover,
    /// Cú nhấn chuột phải được để nguyên cho Windows vì một quy tắc.
    Skip { process: String, reason: String },
    /// Thời gian từ lúc nhận cử chỉ mở tới khung hình đầu tiên của menu.
    Latency { ms: f64 },
    /// Đã nạp lại cài đặt và profile.
    Loaded {
        profiles: usize,
        errors: Vec<String>,
    },
}

impl LogEvent {
    fn describe(&self) -> String {
        let name = |d: &Option<Dir>| d.map_or("giữa", |d| DIR_NAMES[usize::from(d)]);
        match self {
            Self::Open {
                x,
                y,
                dir,
                process,
                profile,
                ..
            } => format!(
                "mở tại ({x}, {y}) · {} → {profile} · hướng {}",
                process.as_deref().unwrap_or("?"),
                name(dir)
            ),
            Self::Dir { dir } => format!("hướng {}", name(dir)),
            Self::Cancel => "huỷ".into(),
            Self::Away => "kéo ra quá xa → đóng menu".into(),
            Self::Empty { dir } => format!("góc {} trống", DIR_NAMES[usize::from(*dir)]),
            Self::Unavailable { label, reason, .. } => format!("{label} đang tắt: {reason}"),
            Self::Run { label, detail, .. } => format!("chạy {label} ({detail})"),
            Self::Error { label, message } => format!("lỗi {label}: {message}"),
            Self::Native => "nhả trước ngưỡng → menu gốc".into(),
            Self::Handover => "nút khác được nhấn → trả cho Windows".into(),
            Self::Skip { process, reason } => format!("bỏ qua {process}: {reason}"),
            Self::Latency { ms } => format!("menu hiện sau {ms:.1} ms"),
            Self::Loaded { profiles, errors } => {
                format!("nạp {profiles} profile, {} lỗi", errors.len())
            }
        }
    }
}

pub(crate) fn log(app: &AppHandle, event: LogEvent) {
    println!("[radial] {}", event.describe());
    let _ = app.emit_to(MAIN_WINDOW, "gesture", event);
}

/// Yêu cầu chụp nền: (số thứ tự lần mở, tâm x, tâm y, cạnh px vật lý, scale).
type BackdropJob = (u64, i32, i32, i32, f64);
/// Yêu cầu đọc icon app: (số thứ tự lần mở, đường dẫn exe, HWND).
type IconJob = (u64, String, isize);

// ---------- Dữ liệu gửi sang cửa sổ menu ----------

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuOpen {
    seq: u64,
    dir: Option<Dir>,
    profile: profiles::MenuProfile,
    /// Icon thật của app (nếu đã có trong bộ đệm); chưa có thì gửi sau qua `menu:icon`.
    app_icon: Option<appicon::AppIcon>,
}

#[derive(Clone, Serialize)]
struct MenuIcon {
    seq: u64,
    #[serde(flatten)]
    icon: appicon::AppIcon,
}

#[derive(Clone, Serialize)]
struct MenuDir {
    dir: Option<Dir>,
}

#[derive(Clone, Serialize)]
struct MenuClose {
    /// "pick" = chọn một hướng, "cancel" = huỷ, "away" = kéo ra quá xa (giữa menu báo "Đóng").
    mode: &'static str,
    dir: Option<Dir>,
}

/// Phần cài đặt mà trang menu cần để tự vẽ.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MenuLook {
    tone: String,
    accent: String,
    /// `system`, `on` hoặc `off`.
    animations: String,
    blur: bool,
    /// Tỉ lệ kích thước menu (1 = mặc định).
    size: f64,
}

impl MenuLook {
    fn from(settings: &Settings) -> Self {
        Self {
            tone: settings.tone.clone(),
            accent: settings.accent.clone(),
            animations: settings.animations.clone(),
            blur: settings.blur,
            size: settings.menu_scale,
        }
    }
}

// ---------- Điều khiển chung ----------

/// Đổi trạng thái tạm dừng và báo cho khay hệ thống lẫn giao diện.
pub fn apply_paused(app: &AppHandle, paused: bool) {
    hook::set_paused(paused);
    tray::sync_paused(app, paused);
    let _ = app.emit("paused", paused);
}

pub fn toggle_paused(app: &AppHandle) {
    apply_paused(app, !hook::is_paused());
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Mở cửa sổ cài đặt từ menu tròn: phải giành quyền active vì app khác đang ở trước.
fn open_settings_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        actions::activate_window(hwnd.0 as isize);
    }
    let _ = window.set_focus();
}

pub fn open_config_folder(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let dir = state.config_dir.clone();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    #[cfg(not(windows))]
    let _ = std::process::Command::new("xdg-open").arg(&dir).spawn();
}

/// Đọc settings.json, thêm profile mặc định mới (nếu có), rồi đọc mọi profile.
fn load_config(config_dir: &std::path::Path) -> (Settings, Store, Vec<String>) {
    let settings_path = config_dir.join("settings.json");
    let profiles_dir = config_dir.join("profiles");
    let (mut settings, settings_error) = Settings::load(&settings_path);
    let mut errors: Vec<String> = settings_error.into_iter().collect();

    let (version, migrate_errors) = profiles::migrate(&profiles_dir, settings.profiles_version);
    errors.extend(migrate_errors);
    if version != settings.profiles_version {
        settings.profiles_version = version;
        if let Err(e) = settings.save(&settings_path) {
            errors.push(format!("Không ghi được settings.json: {e}"));
        }
    }

    let (store, load_errors) = Store::load_dir(&profiles_dir);
    errors.extend(load_errors);
    (settings, store, errors)
}

/// Đọc lại settings.json và mọi profile, áp dụng ngay không cần khởi động lại.
pub fn reload(app: &AppHandle) {
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let (settings, store, errors) = load_config(&state.config_dir);

    apply_settings(app, &settings);
    *state.settings.write().unwrap() = settings;
    let count = store.profiles.len();
    *state.profiles.write().unwrap() = store;
    *state.load_errors.lock().unwrap() = errors.clone();
    log(
        app,
        LogEvent::Loaded {
            profiles: count,
            errors,
        },
    );
    config_changed(app);
}

/// Áp dụng cài đặt mới cho hook, cửa sổ menu và cửa sổ cài đặt.
fn apply_settings(app: &AppHandle, settings: &Settings) {
    hook::set_config(settings.gesture_config());
    overlay::set_capture_exclusion(app, settings.hide_from_capture);
    let _ = app.emit_to(overlay::MENU_WINDOW, "menu:look", MenuLook::from(settings));
    set_window_theme(app, &settings.tone);
}

/// Thanh tiêu đề của cửa sổ cài đặt theo tông sáng/tối đã chọn.
fn set_window_theme(app: &AppHandle, tone: &str) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.set_theme(Some(if tone == "light" {
            Theme::Light
        } else {
            Theme::Dark
        }));
    }
}

/// Báo cửa sổ cài đặt đọc lại trạng thái (profile/cài đặt vừa đổi).
fn config_changed(app: &AppHandle) {
    let _ = app.emit_to(MAIN_WINDOW, "config-changed", ());
}

fn remember_recent(state: &AppState, process: &str) {
    let mut recent = state.recent.lock().unwrap();
    recent.retain(|p| p != process);
    recent.insert(0, process.to_string());
    recent.truncate(RECENT_LIMIT);
}

// ---------- Cử chỉ → menu → hành động ----------

fn handle_effect(app: &AppHandle, effect: Effect) {
    use overlay::MENU_WINDOW;
    let state = app.state::<AppState>();
    match effect {
        Effect::None => {}
        Effect::Open { x, y, scale, dir } => {
            let received = Instant::now();
            let target = state.probe.get().and_then(|p| p.target_at(x, y));
            let process = target
                .as_ref()
                .map(|t| t.process.clone())
                .filter(|p| !p.is_empty());
            let class = target.as_ref().map(|t| t.class.as_str());
            let profile = state
                .profiles
                .read()
                .unwrap()
                .resolve(process.as_deref(), class)
                .clone();
            if let Some(p) = &process {
                remember_recent(&state, p);
            }

            // Nền mờ: nếu menu đã được loại khỏi ảnh chụp thì hiện menu trước rồi chụp
            // song song; nếu không (Windows cũ, hoặc người dùng tắt) phải chụp trước khi
            // menu hiện. Tắt làm mờ thì không chụp gì.
            let (blur, menu_scale) = {
                let s = state.settings.read().unwrap();
                (s.blur, s.menu_scale)
            };
            // Kích thước menu người dùng chọn nhân thêm vào DPI của màn hình: cửa sổ
            // menu, vùng chụp nền và độ mờ đều tính theo kích thước thật trên màn hình.
            let scale = scale * menu_scale;
            let disc = (overlay::DISC_SIZE * scale).round() as i32;
            let capture_now = blur && !overlay::excluded_from_capture();
            let early = capture_now
                .then(|| backdrop::capture(0, x, y, disc, scale))
                .flatten();

            let ctx = Context {
                clipboard: foreground::clipboard_has_data(),
            };
            let seq = overlay::show(app, x, y, scale);
            *state.pending_frame.lock().unwrap() = Some((seq, received));
            let view = profile.view(process.as_deref(), &ctx);

            // Icon thật của app: có sẵn trong bộ đệm thì gửi kèm, chưa thì đọc ở thread riêng.
            let icon_source = target
                .as_ref()
                .filter(|t| appicon::wants_icon(&t.class, &t.path));
            let cached = icon_source.and_then(|t| state.icons.get(&t.path));
            if let (Some(t), None) = (icon_source, &cached) {
                if let Some(tx) = state.icon_jobs.get() {
                    let _ = tx.send((seq, t.path.clone(), t.hwnd));
                }
            }
            let app_icon = cached.flatten();
            let _ = app.emit_to(
                MENU_WINDOW,
                "menu:open",
                MenuOpen {
                    seq,
                    dir,
                    profile: view,
                    app_icon,
                },
            );

            if let Some(mut bd) = early {
                bd.seq = seq;
                let _ = app.emit_to(MENU_WINDOW, "menu:backdrop", bd);
            } else if blur && !capture_now {
                if let Some(tx) = state.backdrop.get() {
                    let _ = tx.send((seq, x, y, disc, scale));
                }
            }

            log(
                app,
                LogEvent::Open {
                    x,
                    y,
                    scale,
                    dir,
                    process,
                    profile: profile.name.clone(),
                },
            );
            *state.session.lock().unwrap() = Some(Session {
                origin: (x, y),
                target,
                profile,
                ctx,
            });
        }
        Effect::Dir(dir) => {
            let _ = app.emit_to(MENU_WINDOW, "menu:dir", MenuDir { dir });
            log(app, LogEvent::Dir { dir });
        }
        Effect::Release(dir) => {
            let session = state.session.lock().unwrap().take();
            let chosen = dir
                .zip(session.as_ref())
                .and_then(|(d, s)| s.profile.slot(usize::from(d)).map(|slot| (d, slot.clone())));

            let blocked = chosen
                .as_ref()
                .zip(session.as_ref())
                .and_then(|((_, slot), s)| slot.unavailable(&s.ctx));
            match (chosen, &session) {
                (Some((d, slot)), Some(_)) if blocked.is_some() => {
                    let _ = app.emit_to(
                        MENU_WINDOW,
                        "menu:close",
                        MenuClose {
                            mode: "cancel",
                            dir: None,
                        },
                    );
                    log(
                        app,
                        LogEvent::Unavailable {
                            dir: d,
                            label: slot.label,
                            reason: blocked.unwrap_or_default().into(),
                        },
                    );
                }
                (Some((d, slot)), Some(s)) => {
                    let _ = app.emit_to(
                        MENU_WINDOW,
                        "menu:close",
                        MenuClose {
                            mode: "pick",
                            dir: Some(d),
                        },
                    );
                    log(
                        app,
                        LogEvent::Run {
                            dir: d,
                            label: slot.label.clone(),
                            detail: slot.action.describe(),
                            process: s.target.as_ref().map(|t| t.process.clone()),
                        },
                    );
                    if slot.action == profiles::Action::Settings {
                        open_settings_window(app);
                    } else if let Some(executor) = state.executor.get() {
                        executor.run(Job {
                            label: slot.label,
                            action: slot.action,
                            click_first: slot.click_first,
                            origin: s.origin,
                            target: s.target.as_ref().map(|t| t.hwnd),
                        });
                    }
                }
                _ => {
                    let _ = app.emit_to(
                        MENU_WINDOW,
                        "menu:close",
                        MenuClose {
                            mode: "cancel",
                            dir: None,
                        },
                    );
                    log(
                        app,
                        dir.map_or(LogEvent::Cancel, |d| LogEvent::Empty { dir: d }),
                    );
                }
            }
            overlay::hide_later(app, overlay::HIDE_DELAY);
        }
        Effect::Away => {
            state.session.lock().unwrap().take();
            let _ = app.emit_to(
                MENU_WINDOW,
                "menu:close",
                MenuClose {
                    mode: "away",
                    dir: None,
                },
            );
            log(app, LogEvent::Away);
            overlay::hide_later(app, overlay::AWAY_HIDE_DELAY);
        }
        Effect::ReplayClick { .. } => log(app, LogEvent::Native),
        Effect::ReplayDown { .. } => log(app, LogEvent::Handover),
    }
}

fn start_engine(app: &AppHandle) {
    let state = app.state::<AppState>();

    let reporter = app.clone();
    let _ = state.executor.set(Executor::start(move |outcome| {
        if let Err(message) = outcome.result {
            log(
                &reporter,
                LogEvent::Error {
                    label: outcome.label,
                    message,
                },
            );
        }
    }));

    let (backdrop_tx, backdrop_rx) = std::sync::mpsc::channel::<BackdropJob>();
    let painter = app.clone();
    std::thread::Builder::new()
        .name("radial-backdrop".into())
        .spawn(move || {
            for (seq, x, y, size, scale) in backdrop_rx {
                if let Some(bd) = backdrop::capture(seq, x, y, size, scale) {
                    let _ = painter.emit_to(overlay::MENU_WINDOW, "menu:backdrop", bd);
                }
            }
        })
        .expect("không tạo được thread radial-backdrop");
    let _ = state.backdrop.set(backdrop_tx);

    let (icon_tx, icon_rx) = std::sync::mpsc::channel::<IconJob>();
    let icons = state.icons.clone();
    let messenger = app.clone();
    std::thread::Builder::new()
        .name("radial-icon".into())
        .spawn(move || {
            for (seq, path, hwnd) in icon_rx {
                if let Some(icon) = icons.load(&path, hwnd) {
                    let _ = messenger.emit_to(
                        overlay::MENU_WINDOW,
                        "menu:icon",
                        MenuIcon { seq, icon },
                    );
                }
            }
        })
        .expect("không tạo được thread radial-icon");
    let _ = state.icon_jobs.set(icon_tx);

    let skipper = app.clone();
    let probe = Probe::spawn(state.settings.clone(), move |target, reason| {
        log(
            &skipper,
            LogEvent::Skip {
                process: target.process.clone(),
                reason: reason.into(),
            },
        );
    });
    let _ = state.probe.set(probe.clone());

    let config = state.settings.read().unwrap().gesture_config();
    let (tx, rx) = std::sync::mpsc::channel::<Effect>();
    let bypass: hook::BypassFn = Box::new(move |x, y| probe.check(x, y));
    match hook::start(config, tx, bypass) {
        Ok(handle) => *state.hook.lock().unwrap() = Some(handle),
        Err(e) => {
            eprintln!("[radial] {e}");
            *state.hook_error.lock().unwrap() = Some(e.to_string());
        }
    }

    // Xử lý hiệu ứng ngoài thread hook để hook luôn nhẹ.
    let app = app.clone();
    std::thread::Builder::new()
        .name("radial-events".into())
        .spawn(move || {
            for effect in rx {
                handle_effect(&app, effect);
            }
        })
        .expect("không tạo được thread radial-events");
}

fn init_state(app: &AppHandle) -> AppState {
    let config_dir = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| std::env::temp_dir().join("RadialMenu"));
    let (settings, store, errors) = load_config(&config_dir);
    for e in &errors {
        eprintln!("[radial] {e}");
    }
    AppState {
        config_dir,
        settings: Arc::new(RwLock::new(settings)),
        profiles: RwLock::new(store),
        load_errors: Mutex::new(errors),
        hook: Mutex::new(None),
        hook_error: Mutex::new(None),
        probe: OnceLock::new(),
        executor: OnceLock::new(),
        session: Mutex::new(None),
        backdrop: OnceLock::new(),
        pending_frame: Mutex::new(None),
        recent: Mutex::new(Vec::new()),
        icons: Arc::new(appicon::IconCache::default()),
        icon_jobs: OnceLock::new(),
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        // Phải đăng ký đầu tiên: mở app lần nữa chỉ đưa cửa sổ đang chạy lên trước.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::save_profile,
            commands::delete_profile,
            commands::save_settings,
            commands::restore_defaults,
            commands::export_backup,
            commands::import_backup,
            commands::list_apps,
            commands::set_paused,
            commands::reload_config,
            commands::open_config_dir,
            commands::menu_look,
            commands::menu_frame
        ])
        .setup(|app| {
            let state = init_state(app.handle());
            let settings = state.settings.read().unwrap().clone();
            app.manage(state);
            // Cài hook trước tiên: đó là phần cửa sổ cài đặt hỏi tới ngay khi tải xong.
            start_engine(app.handle());
            overlay::setup(app.handle(), settings.hide_from_capture)?;
            tray::create(app.handle())?;

            if let Err(e) = autostart::sync(settings.launch_at_login) {
                eprintln!("[radial] {e}");
            }
            set_window_theme(app.handle(), &settings.tone);
            // Trang menu có thể đã hỏi cài đặt trước khi nạp xong: gửi lại cho chắc.
            let _ = app.emit_to(overlay::MENU_WINDOW, "menu:look", MenuLook::from(&settings));
            // Windows mở lúc đăng nhập thì chỉ nằm dưới khay; tự mở thì hiện cài đặt.
            if !autostart::launched_at_login() {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Đóng cửa sổ chỉ ẩn nó đi; app vẫn chạy ở khay hệ thống.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("không khởi động được Radial Menu");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = app.try_state::<AppState>() {
                if let Some(mut handle) = state.hook.lock().unwrap().take() {
                    handle.stop();
                }
            }
        }
    });
}
