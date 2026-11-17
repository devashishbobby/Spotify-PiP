use crate::window_controller::{self as windows, DesktopState};
use std::sync::Mutex;
use tauri::{
    AppHandle, Emitter, Manager,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Show / Hide Player", true, None::<&str>)?;
    let pinned = windows::status(app)
        .map(|status| status.always_on_top)
        .unwrap_or(true);
    let pin = CheckMenuItem::with_id(app, "pin", "Always on top", true, pinned, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let reset = MenuItem::with_id(app, "reset", "Reset position", true, None::<&str>)?;
    let shortcut = windows::status(app)
        .ok()
        .and_then(|s| s.shortcut)
        .unwrap_or_else(|| "Unavailable — use tray".into());
    let hint = MenuItem::with_id(
        app,
        "shortcut",
        format!("Shortcut: {shortcut}"),
        false,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Quit SpotifyPiP", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[&toggle, &pin, &settings, &reset, &hint, &separator, &quit],
    )?;
    if let Ok(mut state) = app.state::<Mutex<DesktopState>>().lock() {
        state.pin_item = Some(pin);
    }
    let icon = app
        .default_window_icon()
        .ok_or_else(|| tauri::Error::AssetNotFound("Application icon".into()))?
        .clone();
    TrayIconBuilder::with_id("player-tray")
        .icon(icon)
        .tooltip(format!(
            "SpotifyPiP · {shortcut}\nRight-click for player controls"
        ))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let result = match event.id.as_ref() {
                "toggle" => windows::toggle(app),
                "pin" => windows::status(app).and_then(|s| windows::set_top(app, !s.always_on_top)),
                "settings" => windows::show(app).and_then(|()| {
                    app.emit("desktop://settings", true)
                        .map_err(|e| e.to_string())
                }),
                "reset" => windows::place(app, true).and_then(|()| windows::show(app)),
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            windows::report(app, result);
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                windows::report(tray.app_handle(), windows::toggle(tray.app_handle()));
            }
        })
        .build(app)?;
    Ok(())
}
