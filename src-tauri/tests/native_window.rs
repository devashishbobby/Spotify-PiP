#![cfg(windows)]
#![allow(dead_code)]
// Compile the same internal modules in an isolated native integration harness.
#[path = "../src/preferences.rs"]
mod preferences;
#[path = "../src/window_controller.rs"]
mod window_controller;
use preferences::*;
use spotify_pip_lib::media_session;
use std::{
    fs,
    sync::{Arc, Mutex},
};
use tauri::Manager;
#[test]
#[ignore = "Requires an interactive Windows desktop and WebView2; briefly shows an isolated test window"]
fn native_window_preferences_smoke() {
    let directory = std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join(format!("native-window-test-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("fixture directory");
    let path = directory.join("preferences.json");
    let test_path = path.clone();
    let result = Arc::new(Mutex::new(None));
    let outcome = result.clone();
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows[0].url =
        tauri::WebviewUrl::External("about:blank".parse().expect("blank test page"));
    context.set_default_window_icon(Some(tauri::image::Image::new_owned(
        vec![32, 32, 32, 255],
        1,
        1,
    )));
    let app = tauri::Builder::default()
        .any_thread()
        .manage(Mutex::new(crate::window_controller::DesktopState::default()))
        .manage(PreferencesStore::open(path.clone()).expect("preferences"))
        .setup(move |app| {
            let checked = (|| -> Result<(), String> {
                let handle = app.handle();
                crate::window_controller::restore(handle)?;
                let window = app.get_webview_window("main").ok_or("No test window")?;
                if window.is_visible().map_err(|e| e.to_string())? {
                    return Err("Startup was not hidden".into());
                }
                let initial = window.outer_position().map_err(|e| e.to_string())?;
                let desired = tauri::PhysicalPosition::new(
                    initial.x.saturating_sub(40),
                    initial.y.saturating_sub(40),
                );
                window.set_position(desired).map_err(|e| e.to_string())?;
                crate::window_controller::set_top(handle, false)?;
                crate::window_controller::capture(handle)?;
                crate::window_controller::show(handle)?;
                if !window.is_visible().map_err(|e| e.to_string())? {
                    return Err("Show failed".into());
                }
                let show_revision = crate::window_controller::motion_ready(handle, false)?.revision;
                crate::window_controller::complete_transition(handle, show_revision)?;
                crate::window_controller::hide(handle)?;
                if !window.is_visible().map_err(|e| e.to_string())? {
                    return Err("Hide bypassed exit acknowledgement".into());
                }
                let old_hide = crate::window_controller::motion_ready(handle, false)?.revision;
                crate::window_controller::show(handle)?;
                crate::window_controller::complete_transition(handle, old_hide)?;
                if !window.is_visible().map_err(|e| e.to_string())? {
                    return Err("Stale exit hid newer Show".into());
                }
                let latest = crate::window_controller::motion_ready(handle, false)?.revision;
                crate::window_controller::complete_transition(handle, latest)?;
                for _ in 0..20 {
                    crate::window_controller::toggle(handle)?;
                }
                if !crate::window_controller::motion_ready(handle, false)?.visible {
                    return Err("Rapid toggle intent mismatch".into());
                }
                crate::window_controller::motion_ready(handle, true)?;
                crate::window_controller::hide(handle)?;
                if window.is_visible().map_err(|e| e.to_string())? {
                    return Err("Hide failed".into());
                }
                app.state::<PreferencesStore>().shutdown();
                let reopened =
                    PreferencesStore::open(test_path.clone()).map_err(|e| e.to_string())?;
                if reopened.snapshot().always_on_top || reopened.snapshot().position.is_none() {
                    return Err("Preferences were not persisted".into());
                }
                reopened.shutdown();
                window.set_always_on_top(true).map_err(|e| e.to_string())?;
                window.set_position(initial).map_err(|e| e.to_string())?;
                crate::window_controller::restore(handle)?;
                if window.is_always_on_top().map_err(|e| e.to_string())?
                    || window.outer_position().map_err(|e| e.to_string())? != desired
                {
                    return Err("Native position/on-top restoration failed".into());
                }
                Ok(())
            })();
            if checked.is_ok() {
                let test_window = app.get_webview_window("main").expect("test window");
                test_window.eval("document.body.innerHTML='<main class=player style=opacity:0></main>'").map_err(std::io::Error::other)?;
                crate::window_controller::motion_ready(app.handle(), true).map_err(std::io::Error::other)?;
                crate::window_controller::show(app.handle()).map_err(std::io::Error::other)?;
                test_window.eval("location.hash=getComputedStyle(document.querySelector('.player')).opacity==='1'?'motion-restored':'motion-hidden'").map_err(std::io::Error::other)?;
                crate::window_controller::hide(app.handle()).map_err(std::io::Error::other)?;
                crate::window_controller::motion_ready(app.handle(), false)
                    .map_err(std::io::Error::other)?;
                crate::window_controller::show(app.handle()).map_err(std::io::Error::other)?;
                crate::window_controller::hide(app.handle()).map_err(std::io::Error::other)?;
            }
            let has_error = checked.is_err();
            *outcome
                .lock()
                .map_err(|_| std::io::Error::other("Test result lock"))? = Some(checked);
            if has_error {
                app.handle().exit(0);
            } else {
                let handle = app.handle().clone();
                let delayed = outcome.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(450)).await;
                    let finish = handle.clone();
                    let result = handle.run_on_main_thread(move || {
                        if finish
                            .get_webview_window("main")
                            .is_some_and(|window| window.is_visible().unwrap_or(true))
                        {
                            *delayed.lock().expect("result lock") =
                                Some(Err("Unresponsive WebView fallback failed to hide".into()));
                        }
                        if finish.get_webview_window("main").and_then(|window| window.url().ok()).is_none_or(|url| url.fragment()!=Some("motion-restored")) {
                            *delayed.lock().expect("result lock") = Some(Err("Reduced-motion missing-listener Show left transparent contents".into()));
                        }
                        crate::window_controller::shutdown(&finish);
                        finish.exit(0);
                    });
                    if result.is_err() {
                        handle.exit(1);
                    }
                });
            }
            Ok(())
        })
        .build(context)
        .expect("build native test app");
    assert_eq!(app.run_return(|_, _| {}), 0);
    let result = result
        .lock()
        .expect("result")
        .take()
        .expect("native smoke ran");
    assert!(result.is_ok(), "{result:?}");
    fs::remove_file(path).expect("remove test preferences");
    fs::remove_dir(directory).expect("remove empty fixture directory");
    println!(
        "Native startup hidden, show/hide, stale acknowledgement, 20 rapid toggles, reduced motion, 280ms fallback, file persistence, position and on-top restoration passed"
    );
}
