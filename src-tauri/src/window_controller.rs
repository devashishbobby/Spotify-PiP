use crate::preferences::{MonitorArea, PreferencesStore, SavedPosition, restore_point};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};

pub struct DesktopState {
    pub always_on_top: bool,
    pub shortcut: Option<String>,
    pub shortcut_error: Option<String>,
    pub positioned: bool,
    motion: MotionState,
    motion_fallback: Option<tauri::async_runtime::JoinHandle<()>>,
    reduced_motion: bool,
    pub pin_item: Option<tauri::menu::CheckMenuItem<tauri::Wry>>,
}
impl Default for DesktopState {
    fn default() -> Self {
        Self {
            always_on_top: true,
            shortcut: None,
            shortcut_error: None,
            positioned: false,
            motion: MotionState::default(),
            motion_fallback: None,
            reduced_motion: false,
            pin_item: None,
        }
    }
}
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStatus {
    pub always_on_top: bool,
    pub shortcut: Option<String>,
    pub shortcut_error: Option<String>,
    pub visible: bool,
}
fn window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "Player window is unavailable".into())
}
pub fn status(app: &AppHandle) -> Result<DesktopStatus, String> {
    let visible = window(app)?.is_visible().map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<DesktopState>>();
    let state = state
        .lock()
        .map_err(|_| "Desktop settings lock unavailable")?;
    Ok(DesktopStatus {
        always_on_top: state.always_on_top,
        shortcut: state.shortcut.clone(),
        shortcut_error: state.shortcut_error.clone(),
        visible,
    })
}
fn publish(app: &AppHandle) -> Result<(), String> {
    if let Some(service) = app.try_state::<crate::media_session::MediaService>() {
        service.set_visible(window(app)?.is_visible().map_err(|e| e.to_string())?);
    }
    app.emit("desktop://status", status(app)?)
        .map_err(|e| e.to_string())
}
pub fn report(app: &AppHandle, result: Result<(), String>) {
    if let Err(error) = result {
        eprintln!("Desktop operation failed: {error}");
        let _ = app.emit("desktop://error", error);
    }
}
pub fn set_top(app: &AppHandle, enabled: bool) -> Result<(), String> {
    window(app)?
        .set_always_on_top(enabled)
        .map_err(|e| e.to_string())?;
    {
        let state = app.state::<Mutex<DesktopState>>();
        let mut state = state
            .lock()
            .map_err(|_| "Desktop settings lock unavailable")?;
        state.always_on_top = enabled;
        if let Some(item) = &state.pin_item {
            item.set_checked(enabled).map_err(|e| e.to_string())?;
        }
    }
    if let Some(preferences) = app.try_state::<PreferencesStore>() {
        preferences.set_top(enabled);
    }
    publish(app)
}
// All callers (shortcut, tray, X, Escape and IPC) share this revisioned intent.
#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowTransition {
    pub revision: u64,
    pub visible: bool,
    pub animate: bool,
    pub was_visible: bool,
}
#[derive(Default)]
struct MotionState {
    revision: u64,
    target: bool,
    pending: bool,
    was_visible: bool,
}
impl MotionState {
    fn desired(&self, actual: bool) -> bool {
        if self.pending { self.target } else { actual }
    }
    fn request(&mut self, visible: bool, actual: bool) -> bool {
        if self.desired(actual) == visible && (self.pending || actual == visible) {
            return false;
        }
        self.revision = self.revision.saturating_add(1);
        self.target = visible;
        self.pending = true;
        self.was_visible = actual;
        true
    }
    fn snapshot(&self, actual: bool, reduced: bool) -> WindowTransition {
        WindowTransition {
            revision: self.revision,
            visible: self.desired(actual),
            animate: self.pending && !reduced,
            was_visible: if self.pending {
                self.was_visible
            } else {
                actual
            },
        }
    }
    fn finish(&mut self, revision: u64) -> Option<bool> {
        if !self.pending || self.revision != revision {
            return None;
        }
        self.pending = false;
        Some(self.target)
    }
}
const MOTION_FALLBACK_MS: u64 = 280;

pub fn motion_ready(app: &AppHandle, reduced: bool) -> Result<WindowTransition, String> {
    let actual = window(app)?.is_visible().map_err(|e| e.to_string())?;
    let (snapshot, complete) = {
        let state = app.state::<Mutex<DesktopState>>();
        let mut state = state
            .lock()
            .map_err(|_| "Desktop settings lock unavailable")?;
        state.reduced_motion = reduced;
        (
            state.motion.snapshot(actual, reduced),
            reduced && state.motion.pending,
        )
    };
    if complete {
        finish_transition(app, snapshot.revision, true)?;
    }
    Ok(snapshot)
}

// Called by the WebView after its exit animation, or by the bounded native fallback.
// A superseded request can never hide a newer Show or release its fallback.
pub fn complete_transition(app: &AppHandle, revision: u64) -> Result<(), String> {
    finish_transition(app, revision, false)
}
fn finish_transition(app: &AppHandle, revision: u64, fallback: bool) -> Result<(), String> {
    let target = {
        let state = app.state::<Mutex<DesktopState>>();
        let mut state = state
            .lock()
            .map_err(|_| "Desktop settings lock unavailable")?;
        let Some(target) = state.motion.finish(revision) else {
            return Ok(());
        };
        if let Some(timer) = state.motion_fallback.take() {
            timer.abort();
        }
        target
    };
    let win = window(app)?;
    if !target {
        if let Err(error) = win.hide() {
            // A refused native Hide must not leave a transparent, focused window.
            let _ = restore_presentation(&win, revision);
            let _ = publish(app);
            return Err(error.to_string());
        }
    } else if fallback {
        restore_presentation(&win, revision)?;
    }
    publish(app)
}

fn restore_presentation(win: &WebviewWindow, revision: u64) -> Result<(), String> {
    // A reload or unavailable frontend must not leave visible native contents
    // transparent. The revision guard also protects queued evaluations.
    win.eval(format!("(()=>{{const p=document.querySelector('.player');if(p&&Number(p.dataset.windowRevision||0)<={revision}){{p.getAnimations().forEach(a=>a.cancel());p.style.opacity='1';p.style.transform='none';p.dataset.windowRevision='{revision}';p.dataset.windowMotion='shown';}}}})()"))
        .map_err(|e| e.to_string())
}

fn transition(app: &AppHandle, visible: bool) -> Result<(), String> {
    let win = window(app)?;
    let actual = win.is_visible().map_err(|e| e.to_string())?;
    let request = {
        let state = app.state::<Mutex<DesktopState>>();
        let mut state = state
            .lock()
            .map_err(|_| "Desktop settings lock unavailable")?;
        if !state.motion.request(visible, actual) {
            None
        } else {
            if let Some(timer) = state.motion_fallback.take() {
                timer.abort();
            }
            Some(state.motion.snapshot(actual, state.reduced_motion))
        }
    };
    if visible {
        let shown = (|| -> Result<(), String> {
            place(app, false)?;
            win.unminimize().map_err(|e| e.to_string())?;
            win.show().map_err(|e| e.to_string())?;
            win.set_focus().map_err(|e| e.to_string())?;
            if let Some(preferences) = app.try_state::<PreferencesStore>()
                && let Some(warning) = preferences.take_warning()
            {
                let _ = app.emit("desktop://error", warning);
            }
            publish(app)?;
            Ok(())
        })();
        if let Err(error) = shown {
            if let Some(request) = &request {
                let state = app.state::<Mutex<DesktopState>>();
                if let Ok(mut state) = state.lock() {
                    state.motion.finish(request.revision);
                }
                if win.is_visible().unwrap_or(false) {
                    let _ = restore_presentation(&win, request.revision);
                }
            }
            let _ = publish(app);
            return Err(error);
        }
    }
    let Some(request) = request else {
        return Ok(());
    };
    let revision = request.revision;
    // The native window is already shown before the entrance can animate.
    let emitted = app.emit("desktop://transition", request.clone());
    if emitted.is_err() || !request.animate {
        finish_transition(app, revision, true)?;
        return emitted.map_err(|e| e.to_string());
    }
    let handle = app.clone();
    let timer = tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(MOTION_FALLBACK_MS)).await;
        let fallback = handle.clone();
        if let Err(error) = handle.run_on_main_thread(move || {
            report(&fallback, finish_transition(&fallback, revision, true));
        }) {
            eprintln!("Cannot complete window transition: {error}");
        }
    });
    let state = app.state::<Mutex<DesktopState>>();
    let mut state = state
        .lock()
        .map_err(|_| "Desktop settings lock unavailable")?;
    // A synchronous acknowledgement or newer request may have won already.
    if state.motion.pending && state.motion.revision == revision {
        state.motion_fallback = Some(timer);
    } else {
        timer.abort();
    }
    Ok(())
}
pub fn hide(app: &AppHandle) -> Result<(), String> {
    // Persistence must never prevent the player from being hidden.
    report(app, capture(app));
    transition(app, false)
}
pub fn show(app: &AppHandle) -> Result<(), String> {
    transition(app, true)
}
pub fn toggle(app: &AppHandle) -> Result<(), String> {
    let actual = window(app)?.is_visible().map_err(|e| e.to_string())?;
    let desired = app
        .state::<Mutex<DesktopState>>()
        .lock()
        .map_err(|_| "Desktop settings lock unavailable")?
        .motion
        .desired(actual);
    if desired { hide(app) } else { show(app) }
}
pub fn shutdown(app: &AppHandle) {
    if let Ok(mut state) = app.state::<Mutex<DesktopState>>().lock() {
        if let Some(timer) = state.motion_fallback.take() {
            timer.abort();
        }
        state.motion.pending = false;
    }
}
// Preserve toggle behavior and clamp to a current monitor's work area on every show.
pub fn place(app: &AppHandle, reset: bool) -> Result<(), String> {
    let win = window(app)?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let position = win.outer_position().map_err(|e| e.to_string())?;
    let monitors = win.available_monitors().map_err(|e| e.to_string())?;
    let state = app.state::<Mutex<DesktopState>>();
    let mut state = state
        .lock()
        .map_err(|_| "Desktop settings lock unavailable")?;
    let monitor = if reset || !state.positioned {
        win.primary_monitor().map_err(|e| e.to_string())?
    } else {
        monitors
            .iter()
            .find(|monitor| {
                let area = monitor.work_area();
                let center_x = i64::from(position.x) + i64::from(size.width) / 2;
                let center_y = i64::from(position.y) + i64::from(size.height) / 2;
                center_x >= i64::from(area.position.x)
                    && center_x < i64::from(area.position.x) + i64::from(area.size.width)
                    && center_y >= i64::from(area.position.y)
                    && center_y < i64::from(area.position.y) + i64::from(area.size.height)
            })
            .cloned()
            .or(win.primary_monitor().map_err(|e| e.to_string())?)
    }
    .ok_or("No monitor is currently available")?;
    let area = monitor.work_area();
    let margin = (20.0 * monitor.scale_factor()).round() as i32;
    let min_x = area.position.x;
    let min_y = area.position.y;
    let max_x = (i64::from(min_x) + i64::from(area.size.width) - i64::from(size.width))
        .max(i64::from(min_x)) as i32;
    let max_y = (i64::from(min_y) + i64::from(area.size.height) - i64::from(size.height))
        .max(i64::from(min_y)) as i32;
    let target = if reset || !state.positioned {
        PhysicalPosition::new((max_x - margin).max(min_x), (max_y - margin).max(min_y))
    } else {
        PhysicalPosition::new(
            position.x.clamp(min_x, max_x),
            position.y.clamp(min_y, max_y),
        )
    };
    win.set_position(target).map_err(|e| e.to_string())?;
    state.positioned = true;
    Ok(())
}

fn monitor_areas(win: &WebviewWindow) -> Result<Vec<MonitorArea>, String> {
    let primary = win.primary_monitor().map_err(|e| e.to_string())?;
    Ok(win
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|monitor| {
            let area = monitor.work_area();
            MonitorArea {
                name: monitor.name().cloned(),
                x: area.position.x,
                y: area.position.y,
                width: area.size.width,
                height: area.size.height,
                scale: monitor.scale_factor(),
                primary: primary.as_ref().is_some_and(|p| {
                    p.position() == monitor.position() && p.size() == monitor.size()
                }),
            }
        })
        .collect())
}
pub fn capture(app: &AppHandle) -> Result<(), String> {
    let Some(preferences) = app.try_state::<PreferencesStore>() else {
        return Ok(());
    };
    let win = window(app)?;
    let point = win.outer_position().map_err(|e| e.to_string())?;
    let size = win.outer_size().map_err(|e| e.to_string())?;
    let areas = monitor_areas(&win)?;
    let center = (
        i64::from(point.x) + i64::from(size.width) / 2,
        i64::from(point.y) + i64::from(size.height) / 2,
    );
    let area = areas.iter().find(|area| {
        center.0 >= i64::from(area.x)
            && center.0 < i64::from(area.x) + i64::from(area.width)
            && center.1 >= i64::from(area.y)
            && center.1 < i64::from(area.y) + i64::from(area.height)
    });
    if let Some(area) = area
        && area.scale.is_finite()
        && area.scale > 0.0
    {
        preferences.remember(SavedPosition {
            monitor: area.name.clone(),
            offset_x: (f64::from(point.x) - f64::from(area.x)) / area.scale,
            offset_y: (f64::from(point.y) - f64::from(area.y)) / area.scale,
        });
    }
    Ok(())
}
pub fn restore(app: &AppHandle) -> Result<(), String> {
    let preferences = app.state::<PreferencesStore>().snapshot();
    set_top(app, preferences.always_on_top)?;
    let win = window(app)?;
    if let Some(saved) = preferences.position {
        let size = win.outer_size().map_err(|e| e.to_string())?;
        let scale = win.scale_factor().map_err(|e| e.to_string())?;
        let logical_size = (
            f64::from(size.width) / scale,
            f64::from(size.height) / scale,
        );
        if let Some((x, y)) = restore_point(&saved, &monitor_areas(&win)?, logical_size) {
            win.set_position(PhysicalPosition::new(x, y))
                .map_err(|e| e.to_string())?;
            app.state::<Mutex<DesktopState>>()
                .lock()
                .map_err(|_| "Desktop settings lock unavailable")?
                .positioned = true;
        }
    }
    place(app, false)?;
    capture(app)
}

#[cfg(test)]
mod motion_tests {
    use super::MotionState;
    #[test]
    fn rapid_reversal_ignores_stale_completion_and_uses_intent() {
        let mut state = MotionState::default();
        assert!(state.request(true, false));
        let showing = state.revision;
        assert!(state.request(false, true));
        let hiding = state.revision;
        assert!(!state.desired(true));
        assert!(state.request(true, true));
        let reshowing = state.revision;
        assert_eq!(state.finish(showing), None);
        assert_eq!(state.finish(hiding), None);
        assert_eq!(state.finish(reshowing), Some(true));
        assert_eq!(state.finish(reshowing), None);
    }
    #[test]
    fn repeated_requests_are_idempotent_and_reduced_motion_is_static() {
        let mut state = MotionState::default();
        assert!(!state.request(false, false));
        assert!(state.request(true, false));
        let revision = state.revision;
        assert!(!state.request(true, true));
        assert_eq!(state.revision, revision);
        assert!(!state.snapshot(true, true).animate);
        assert!(state.snapshot(true, false).animate);
        assert_eq!(state.finish(revision), Some(true));
        assert!(!state.snapshot(true, false).animate);
    }
}
