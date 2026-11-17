pub mod media_session;
mod preferences;
mod tray;
mod window_controller;

use media_session::{
    MediaService,
    state::{AppError, CommandResult, MediaAction, MediaSnapshot},
};
use preferences::PreferencesStore;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use window_controller::{DesktopState, DesktopStatus};
struct SecondLaunch(AtomicBool);

#[tauri::command]
async fn media_play_pause(
    service: tauri::State<'_, MediaService>,
    expected_session: u64,
) -> Result<CommandResult, AppError> {
    service
        .execute(MediaAction::PlayPause, expected_session)
        .await
}
#[tauri::command]
async fn media_next(
    service: tauri::State<'_, MediaService>,
    expected_session: u64,
) -> Result<CommandResult, AppError> {
    service.execute(MediaAction::Next, expected_session).await
}
#[tauri::command]
async fn media_previous(
    service: tauri::State<'_, MediaService>,
    expected_session: u64,
) -> Result<CommandResult, AppError> {
    service
        .execute(MediaAction::Previous, expected_session)
        .await
}
#[tauri::command]
async fn media_seek(
    service: tauri::State<'_, MediaService>,
    expected_session: u64,
    position_ms: i64,
) -> Result<CommandResult, AppError> {
    service
        .execute(MediaAction::Seek(position_ms), expected_session)
        .await
}
#[tauri::command]
async fn media_set_volume(
    service: tauri::State<'_, MediaService>,
    expected_session: u64,
    level: f32,
) -> Result<CommandResult, AppError> {
    service
        .execute(MediaAction::Volume(level), expected_session)
        .await
}
#[tauri::command]
async fn media_set_muted(
    service: tauri::State<'_, MediaService>,
    expected_session: u64,
    muted: bool,
) -> Result<CommandResult, AppError> {
    service
        .execute(MediaAction::Mute(muted), expected_session)
        .await
}

#[tauri::command]
fn get_media_snapshot(service: tauri::State<'_, MediaService>) -> Result<MediaSnapshot, AppError> {
    service.snapshot()
}
#[tauri::command]
async fn media_refresh(service: tauri::State<'_, MediaService>) -> Result<MediaSnapshot, AppError> {
    service.refresh().await
}

#[tauri::command]
fn get_desktop_status(app: tauri::AppHandle) -> Result<DesktopStatus, String> {
    window_controller::status(&app)
}
#[tauri::command]
fn hide_player(app: tauri::AppHandle) -> Result<(), String> {
    window_controller::hide(&app)
}
#[tauri::command]
fn window_motion_ready(
    app: tauri::AppHandle,
    reduced_motion: bool,
) -> Result<window_controller::WindowTransition, String> {
    window_controller::motion_ready(&app, reduced_motion)
}
#[tauri::command]
fn window_transition_complete(app: tauri::AppHandle, revision: u64) -> Result<(), String> {
    window_controller::complete_transition(&app, revision)
}
#[tauri::command]
fn set_always_on_top(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    window_controller::set_top(&app, enabled)
}
#[tauri::command]
fn reset_window_position(app: tauri::AppHandle) -> Result<(), String> {
    window_controller::place(&app, true)
}

pub fn run() {
    let builder = tauri::Builder::default()
        .manage(Mutex::new(DesktopState::default()))
        .manage(SecondLaunch(AtomicBool::new(false)));
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| {
        app.state::<SecondLaunch>().0.store(true, Ordering::Relaxed);
        let handle = app.clone();
        let result = app.run_on_main_thread(move || {
            if handle.get_webview_window("main").is_some() {
                handle
                    .state::<SecondLaunch>()
                    .0
                    .store(false, Ordering::Relaxed);
                window_controller::report(&handle, window_controller::show(&handle));
            }
        });
        if let Err(error) = result {
            eprintln!("Could not restore existing player: {error}");
        }
    }));
    let result = builder
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, _, event| {
            if event.state() == ShortcutState::Pressed {
                let handle = app.clone();
                if let Err(error) = app.run_on_main_thread(move || {
                    window_controller::report(&handle, window_controller::toggle(&handle));
                }) { eprintln!("Cannot dispatch shortcut: {error}"); }
            }
        }).build())
        .invoke_handler(tauri::generate_handler![get_desktop_status, hide_player, window_motion_ready, window_transition_complete, set_always_on_top, reset_window_position, get_media_snapshot, media_refresh, media_play_pause, media_next, media_previous, media_seek, media_set_volume, media_set_muted])
        .setup(|app| {
            app.manage(PreferencesStore::open(app.path().app_config_dir()?.join("window-preferences.json"))?);
            window_controller::restore(app.handle()).map_err(std::io::Error::other)?;
            app.manage(MediaService::start(app.handle().clone())?);
            // Register once per process, never in the React/WebView lifecycle.
            let primary = app.global_shortcut().register("Ctrl+Alt+M");
            let (shortcut, shortcut_error) = match primary {
                Ok(()) => (Some("Ctrl+Alt+M".to_string()), None),
                Err(error) => {
                    let fallback = app.global_shortcut().register("Ctrl+Alt+Shift+M");
                    match fallback {
                        Ok(()) => (Some("Ctrl+Alt+Shift+M".to_string()), Some(format!("Ctrl+Alt+M is unavailable ({error}). Use Ctrl+Alt+Shift+M or the tray menu."))),
                        Err(fallback_error) => (None, Some(format!("Shortcuts unavailable: {error}; {fallback_error}. Use the tray Show / Hide Player action."))),
                    }
                }
            };
            let state = app.state::<Mutex<DesktopState>>();
            let mut state = state.lock().map_err(|_| "Desktop settings lock unavailable")?;
            state.shortcut = shortcut;
            state.shortcut_error = shortcut_error;
            drop(state);
            tray::create(app.handle())?;
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        window_controller::report(&handle, window_controller::hide(&handle));
                    }
                    if matches!(event,tauri::WindowEvent::Moved(_)) {
                        window_controller::report(&handle,window_controller::capture(&handle));
                    }
                    if matches!(event,tauri::WindowEvent::ScaleFactorChanged {..}) {
                        window_controller::report(&handle,window_controller::place(&handle,false).and_then(|()|window_controller::capture(&handle)));
                    }
                });
            }
            // Conflict is also available permanently in Settings and the tray status item.
            if window_controller::status(app.handle()).is_ok_and(|s| s.shortcut_error.is_some()) {
                window_controller::show(app.handle()).map_err(std::io::Error::other)?;
                let _ = app.emit("desktop://settings", true);
            }
            #[cfg(debug_assertions)]
            {
                let status = window_controller::status(app.handle()).map_err(std::io::Error::other)?;
                eprintln!("SpotifyPiP ready: tray created, shortcut={:?}, visible={}, always_on_top={}", status.shortcut, status.visible, status.always_on_top);
                if let Some(error) = status.shortcut_error { eprintln!("{error}"); }
            }
            if app.state::<SecondLaunch>().0.swap(false,Ordering::Relaxed) {window_controller::show(app.handle()).map_err(std::io::Error::other)?;}
            Ok(())
        })
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|handle, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                window_controller::report(handle, window_controller::capture(handle));
            }
            if matches!(event, tauri::RunEvent::Exit) {
                window_controller::shutdown(handle);
                handle.state::<MediaService>().shutdown();
                handle.state::<PreferencesStore>().shutdown();
            }
            if matches!(event, tauri::RunEvent::Exit)
                && let Err(error) = handle.global_shortcut().unregister_all()
            {
                eprintln!("Shortcut cleanup failed: {error}");
            }
        }),
        Err(error) => {
            eprintln!("SpotifyPiP could not start: {error}");
            std::process::exit(1);
        }
    }
}
