//! Icon ở khay hệ thống: mở cài đặt, tạm dừng, cấu hình, thoát, và báo bản mới.

use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

use tauri::{AppHandle, Emitter, Manager, Wry};

struct TrayItems {
    pause: CheckMenuItem<Wry>,
    menu: Menu<Wry>,
    /// Mục "Cập nhật lên …", chỉ có khi tìm thấy bản mới.
    update: Mutex<Option<MenuItem<Wry>>>,
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Mở cài đặt", true, None::<&str>)?;
    let pause = CheckMenuItem::with_id(
        app,
        "pause",
        "Tạm dừng",
        true,
        crate::hook::is_paused(),
        None::<&str>,
    )?;
    let folder = MenuItem::with_id(app, "folder", "Mở thư mục cấu hình", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Nạp lại cấu hình", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Thoát Radial Menu", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &pause, &sep1, &folder, &reload, &sep2, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("Radial Menu")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => crate::show_main_window(app),
            "pause" => crate::toggle_paused(app),
            "folder" => crate::open_config_folder(app),
            "reload" => crate::reload(app),
            "update" => {
                crate::show_main_window(app);
                let _ = app.emit_to(crate::MAIN_WINDOW, "update:open", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    app.manage(TrayItems {
        pause,
        menu,
        update: Mutex::new(None),
    });
    Ok(())
}

/// Có bản mới: thêm mục "Cập nhật lên …" lên đầu menu khay và ghi vào tooltip.
pub fn show_update(app: &AppHandle, version: &str) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    let text = format!("Cập nhật lên {version}…");
    let mut slot = items.update.lock().unwrap();
    match slot.as_ref() {
        Some(item) => {
            let _ = item.set_text(&text);
        }
        None => {
            if let Ok(item) = MenuItem::with_id(app, "update", &text, true, None::<&str>) {
                if items.menu.insert(&item, 0).is_ok() {
                    *slot = Some(item);
                }
            }
        }
    }
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(format!("Radial Menu · có bản mới {version}")));
    }
}

/// Không còn bản mới (vừa hỏi lại): bỏ mục cập nhật khỏi menu khay.
pub fn hide_update(app: &AppHandle) {
    let Some(items) = app.try_state::<TrayItems>() else {
        return;
    };
    if let Some(item) = items.update.lock().unwrap().take() {
        let _ = items.menu.remove(&item);
    }
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some("Radial Menu"));
    }
}

/// Giữ dấu tích "Tạm dừng" khớp với trạng thái thật.
pub fn sync_paused(app: &AppHandle, paused: bool) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.pause.set_checked(paused);
    }
}
