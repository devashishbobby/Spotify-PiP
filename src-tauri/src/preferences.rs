use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedPosition {
    pub monitor: Option<String>,
    pub offset_x: f64,
    pub offset_y: f64,
}
impl SavedPosition {
    fn valid(&self) -> bool {
        self.offset_x.is_finite()
            && self.offset_y.is_finite()
            && self.offset_x.abs() <= 100_000.0
            && self.offset_y.abs() <= 100_000.0
            && self.monitor.as_ref().is_none_or(|name| name.len() <= 128)
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub version: u8,
    pub always_on_top: bool,
    pub position: Option<SavedPosition>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            always_on_top: true,
            position: None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct MonitorArea {
    pub name: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}
fn usable(area: &MonitorArea) -> bool {
    area.width > 0
        && area.height > 0
        && area.scale.is_finite()
        && area.scale > 0.0
        && area.scale <= 16.0
}
pub fn restore_point(
    saved: &SavedPosition,
    monitors: &[MonitorArea],
    logical_size: (f64, f64),
) -> Option<(i32, i32)> {
    if !saved.valid()
        || !logical_size.0.is_finite()
        || !logical_size.1.is_finite()
        || !(1.0..=100_000.0).contains(&logical_size.0)
        || !(1.0..=100_000.0).contains(&logical_size.1)
    {
        return None;
    }
    let preferred = monitors
        .iter()
        .filter(|area| usable(area))
        .find(|area| saved.monitor.is_some() && area.name == saved.monitor);
    let monitor = preferred
        .or_else(|| monitors.iter().find(|area| usable(area) && area.primary))
        .or_else(|| monitors.iter().find(|area| usable(area)))?;
    let max_x = (i64::from(monitor.x) + i64::from(monitor.width)
        - (logical_size.0.max(1.0) * monitor.scale).round() as i64)
        .max(i64::from(monitor.x));
    let max_y = (i64::from(monitor.y) + i64::from(monitor.height)
        - (logical_size.1.max(1.0) * monitor.scale).round() as i64)
        .max(i64::from(monitor.y));
    let (x, y) = if preferred.is_none() && saved.monitor.is_some() {
        (
            max_x - (20.0 * monitor.scale).round() as i64,
            max_y - (20.0 * monitor.scale).round() as i64,
        )
    } else {
        (
            i64::from(monitor.x) + (saved.offset_x * monitor.scale).round() as i64,
            i64::from(monitor.y) + (saved.offset_y * monitor.scale).round() as i64,
        )
    };
    Some((
        x.clamp(i64::from(monitor.x), max_x)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
        y.clamp(i64::from(monitor.y), max_y)
            .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32,
    ))
}

struct Shared {
    preferences: Preferences,
    revision: u64,
    changed: Instant,
    warning: Option<String>,
}
pub struct PreferencesStore {
    shared: Arc<Mutex<Shared>>,
    wake: SyncSender<()>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
fn load(path: &PathBuf) -> (Preferences, Option<String>) {
    let result: std::io::Result<Preferences> = (|| {
        let file = File::open(path)?;
        let mut bytes = Vec::new();
        file.take(16_385).read_to_end(&mut bytes)?;
        if bytes.len() > 16_384 {
            return Err(std::io::Error::other("Preferences file is too large"));
        }
        let prefs: Preferences = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
        if prefs.version != 1
            || prefs
                .position
                .as_ref()
                .is_some_and(|position| !position.valid())
        {
            return Err(std::io::Error::other("Invalid preferences"));
        }
        Ok(prefs)
    })();
    match result {
        Ok(prefs) => (prefs, None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (Preferences::default(), None)
        }
        Err(_) => (
            Preferences::default(),
            Some("Saved window preferences could not be read. Defaults were restored.".into()),
        ),
    }
}
fn save(path: &PathBuf, preferences: &Preferences) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(preferences).map_err(std::io::Error::other)?;
    let mut file = File::create(&temporary)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)
}
fn persist_latest(path: &PathBuf, shared: &Mutex<Shared>, saved_revision: &mut u64) {
    let next = shared
        .lock()
        .ok()
        .map(|state| (state.preferences.clone(), state.revision));
    if let Some((preferences, revision)) = next
        && revision != *saved_revision
    {
        match save(path, &preferences) {
            Ok(()) => {
                *saved_revision = revision;
            }
            Err(_) => {
                if let Ok(mut state) = shared.lock() {
                    state.warning = Some(
                        "Window preferences could not be saved. Changes may be lost on restart."
                            .into(),
                    );
                    eprintln!("SpotifyPiP: window preferences could not be saved");
                }
            }
        }
    }
}
impl PreferencesStore {
    pub fn open(path: PathBuf) -> std::io::Result<Self> {
        let (preferences, warning) = load(&path);
        let shared = Arc::new(Mutex::new(Shared {
            preferences,
            revision: 0,
            changed: Instant::now(),
            warning,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let (wake, receive) = mpsc::sync_channel(1);
        let (inner, stopped) = (shared.clone(), stop.clone());
        let worker = std::thread::Builder::new()
            .name("window-preferences".into())
            .spawn(move || {
                let mut saved_revision = 0;
                while receive.recv().is_ok() {
                    loop {
                        let delay = inner
                            .lock()
                            .map(|state| {
                                Duration::from_millis(350).saturating_sub(state.changed.elapsed())
                            })
                            .unwrap_or(Duration::ZERO);
                        if stopped.load(Ordering::Relaxed) || delay.is_zero() {
                            break;
                        }
                        match receive.recv_timeout(delay) {
                            Ok(()) => {}
                            Err(_) => break,
                        }
                    }
                    persist_latest(&path, &inner, &mut saved_revision);
                    if stopped.load(Ordering::Relaxed) {
                        persist_latest(&path, &inner, &mut saved_revision);
                        break;
                    }
                }
            })?;
        Ok(Self {
            shared,
            wake,
            stop,
            worker: Mutex::new(Some(worker)),
        })
    }
    pub fn snapshot(&self) -> Preferences {
        self.shared
            .lock()
            .map(|state| state.preferences.clone())
            .unwrap_or_default()
    }
    fn update(&self, update: impl FnOnce(&mut Preferences)) {
        if let Ok(mut state) = self.shared.lock() {
            let before = state.preferences.clone();
            update(&mut state.preferences);
            if before == state.preferences {
                return;
            }
            state.revision = state.revision.saturating_add(1);
            state.changed = Instant::now();
            drop(state);
            let _ = self.wake.try_send(());
        }
    }
    pub fn set_top(&self, enabled: bool) {
        self.update(|preferences| preferences.always_on_top = enabled);
    }
    pub fn remember(&self, position: SavedPosition) {
        if position.valid() {
            self.update(|preferences| preferences.position = Some(position));
        }
    }
    pub fn take_warning(&self) -> Option<String> {
        self.shared
            .lock()
            .ok()
            .and_then(|mut state| state.warning.take())
    }
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.wake.try_send(());
        if let Ok(mut worker) = self.worker.lock()
            && let Some(worker) = worker.take()
        {
            let _ = worker.join();
        }
    }
}
impl Drop for PreferencesStore {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor(name: &str, x: i32, scale: f64) -> MonitorArea {
        MonitorArea {
            name: Some(name.into()),
            x,
            y: 0,
            width: 1920,
            height: 1080,
            scale,
            primary: x == 0,
        }
    }
    #[test]
    fn restores_logical_offsets_after_dpi_and_monitor_changes() {
        let saved = SavedPosition {
            monitor: Some("secondary".into()),
            offset_x: 100.0,
            offset_y: 200.0,
        };
        assert_eq!(
            restore_point(
                &saved,
                &[monitor("primary", 0, 1.0), monitor("secondary", 1920, 1.5)],
                (360.0, 208.0)
            ),
            Some((2070, 300))
        );
        assert_eq!(
            restore_point(&saved, &[monitor("primary", 0, 1.0)], (360.0, 208.0)),
            Some((1540, 852))
        );
        let offscreen = SavedPosition {
            monitor: Some("primary".into()),
            offset_x: 50000.0,
            offset_y: -200.0,
        };
        assert_eq!(
            restore_point(&offscreen, &[monitor("primary", 0, 1.0)], (360.0, 208.0)),
            Some((1560, 0))
        );
    }
    #[test]
    fn invalid_geometry_is_ignored() {
        let saved = SavedPosition {
            monitor: None,
            offset_x: f64::NAN,
            offset_y: 0.0,
        };
        assert!(restore_point(&saved, &[monitor("primary", 0, 1.0)], (360.0, 208.0)).is_none());
    }
    #[test]
    fn atomic_preferences_round_trip_and_shutdown_flush_latest_changes() {
        let directory = std::env::current_dir()
            .expect("cwd")
            .join("target")
            .join(format!(
                "prefs-test-{}-{}",
                std::process::id(),
                super::super::media_session::state::utc_ms()
            ));
        fs::create_dir_all(&directory).expect("create");
        let path = directory.join("preferences.json");
        let store = PreferencesStore::open(path.clone()).expect("open");
        for x in 0..100 {
            store.remember(SavedPosition {
                monitor: Some("primary".into()),
                offset_x: f64::from(x),
                offset_y: 200.0,
            });
        }
        store.set_top(false);
        store.shutdown();
        let (loaded, warning) = load(&path);
        assert!(warning.is_none());
        assert!(!loaded.always_on_top);
        assert_eq!(loaded.position.map(|p| p.offset_x), Some(99.0));
        let store = PreferencesStore::open(path.clone()).expect("reopen");
        store.set_top(true);
        store.shutdown();
        assert!(load(&path).0.always_on_top);
        fs::write(&path, b"invalid json").expect("corrupt fixture");
        assert!(load(&path).1.is_some());
        fs::remove_file(path).expect("remove fixture");
        fs::remove_dir(directory).expect("remove empty fixture directory");
    }
}
