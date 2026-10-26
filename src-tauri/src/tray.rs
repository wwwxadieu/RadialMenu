//! Icon ở khay hệ thống: mở cài đặt, tạm dừng, cấu hình, thoát.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

struct TrayItems {
    pause: CheckMenuItem<Wry>,
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

    app.manage(TrayItems { pause });
    Ok(())
}

/// Giữ dấu tích "Tạm dừng" khớp với trạng thái thật.
pub fn sync_paused(app: &AppHandle, paused: bool) {
    if let Some(items) = app.try_state::<TrayItems>() {
        let _ = items.pause.set_checked(paused);
    }
}
