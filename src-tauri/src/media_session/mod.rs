#[cfg(windows)]
mod artwork;
#[cfg(windows)]
mod audio_volume;
mod diagnostics;
pub mod state;
mod timeline;
#[cfg(windows)]
mod windows;

use state::{AppError, CommandResult, MediaAction, MediaSnapshot, MediaStatus};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};

pub trait MediaBackend {
    fn refresh(&mut self, force_metadata: bool) -> Result<MediaSnapshot, AppError>;
    fn refresh_fast(&mut self, force_metadata: bool) -> Result<MediaSnapshot, AppError> {
        self.refresh(force_metadata)
    }
    fn execute(
        &mut self,
        action: MediaAction,
        expected_session: u64,
    ) -> Result<CommandResult, AppError>;
    fn event_count(&self) -> u64 {
        0
    }
}
enum Request {
    Refresh(tokio::sync::oneshot::Sender<Result<MediaSnapshot, AppError>>),
    Action(
        MediaAction,
        u64,
        tokio::sync::oneshot::Sender<Result<CommandResult, AppError>>,
    ),
    Wake,
    Stop,
}
#[derive(Clone)]
pub(super) struct RefreshSignal {
    sender: SyncSender<Request>,
    dirty: Arc<AtomicU8>,
    count: Arc<AtomicU64>,
}
impl RefreshSignal {
    pub fn notify(&self, flags: u8) {
        self.count.fetch_add(1, Ordering::Relaxed);
        if self.dirty.fetch_or(flags, Ordering::Relaxed) == 0 {
            let _ = self.sender.try_send(Request::Wake);
        }
    }
}
pub struct MediaService {
    snapshot: Arc<Mutex<MediaSnapshot>>,
    sender: SyncSender<Request>,
    visible: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl MediaService {
    pub fn start(app: AppHandle) -> std::io::Result<Self> {
        let snapshot = Arc::new(Mutex::new(MediaSnapshot::empty(MediaStatus::Loading)));
        let visible = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::sync_channel(32);
        let signal = RefreshSignal {
            sender: sender.clone(),
            dirty: Arc::new(AtomicU8::new(0)),
            count: Arc::new(AtomicU64::new(0)),
        };
        let (shared, shown, stopped) = (snapshot.clone(), visible.clone(), stop.clone());
        let worker = std::thread::Builder::new()
            .name("spotify-media".into())
            .spawn(move || {
                let mut backend: Option<Box<dyn MediaBackend>> = None;
                let generation = Arc::new(AtomicU64::new(0));
                let mut first = true;
                let mut last_refresh = Instant::now();
                while !stopped.load(Ordering::Relaxed) {
                    let active = read(&shared).is_ok_and(|s| s.source == "spotify");
                    let interval = if !active {
                        Duration::from_secs(5)
                    } else if shown.load(Ordering::Relaxed) {
                        Duration::from_millis(1500)
                    } else {
                        Duration::from_secs(10)
                    };
                    let delay = interval.saturating_sub(last_refresh.elapsed());
                    let request = if first {
                        first = false;
                        None
                    } else {
                        match receiver.recv_timeout(delay) {
                            Ok(Request::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                            Ok(request) => Some(request),
                            Err(mpsc::RecvTimeoutError::Timeout) => None,
                        }
                    };
                    if matches!(request, Some(Request::Wake)) {
                        std::thread::sleep(Duration::from_millis(40));
                    }
                    let flags = signal.dirty.swap(0, Ordering::Relaxed);
                    // Timeline-only event storms cannot drive hidden React/native updates at high frequency.
                    if matches!(request, Some(Request::Wake))
                        && flags == 1
                        && last_refresh.elapsed() < interval
                    {
                        continue;
                    }
                    let metadata_dirty = flags & 4 != 0;
                    let mut refresh = |force| {
                        if backend.is_none() {
                            match create_backend(
                                stopped.clone(),
                                generation.clone(),
                                signal.clone(),
                            ) {
                                Ok(service) => backend = Some(service),
                                Err(error) => {
                                    publish(&app, &shared, MediaSnapshot::failure(&error));
                                    return Err(error);
                                }
                            }
                        }
                        let next = match backend.as_mut() {
                            Some(service) => service.refresh(force),
                            None => Err(AppError::new(
                                "media_unavailable",
                                "Windows media sessions could not be initialized. Please retry.",
                            )),
                        };
                        match next {
                            Ok(next) => {
                                publish(&app, &shared, next.clone());
                                Ok(read(&shared)?)
                            }
                            Err(error) => {
                                publish(&app, &shared, MediaSnapshot::failure(&error));
                                backend = None;
                                Err(error)
                            }
                        }
                    };
                    match request {
                        Some(Request::Refresh(response)) => {
                            let _ = response.send(refresh(true));
                        }
                        Some(Request::Action(action, expected, response)) => {
                            drop(refresh);
                            let result = if let Some(service) = backend.as_mut() {
                                let mut result =
                                    execute_confirmed(&mut **service, action, expected, &stopped);
                                match &mut result {
                                    Ok(result) => {
                                        if let Some(next) = result.snapshot.take() {
                                            publish(&app, &shared, next);
                                        }
                                        result.snapshot = read(&shared).ok();
                                    }
                                    Err(_) => match service.refresh_fast(false) {
                                        Ok(next) => publish(&app, &shared, next),
                                        Err(error) => {
                                            publish(&app, &shared, MediaSnapshot::failure(&error))
                                        }
                                    },
                                }
                                result
                            } else {
                                Err(AppError::new(
                                    "no_session",
                                    "No Spotify session is available.",
                                ))
                            };
                            let _ = response.send(result);
                        }
                        Some(Request::Stop) => break,
                        Some(Request::Wake) | None => {
                            let _ = refresh(metadata_dirty);
                        }
                    }
                    last_refresh = Instant::now();
                }
                drop(backend); // Release WinRT references and registrations on the owning worker.
            })?;
        Ok(Self {
            snapshot,
            sender,
            visible,
            stop,
            worker: Mutex::new(Some(worker)),
        })
    }
    pub fn snapshot(&self) -> Result<MediaSnapshot, AppError> {
        read(&self.snapshot)
    }
    pub async fn refresh(&self) -> Result<MediaSnapshot, AppError> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.sender
            .try_send(Request::Refresh(sender))
            .map_err(|_| AppError::new("busy", "The media service is busy. Please retry."))?;
        receiver
            .await
            .map_err(|_| AppError::new("stopped", "The media service has stopped."))?
    }
    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Ordering::Relaxed);
        if visible {
            let _ = self.sender.try_send(Request::Wake);
        }
    }
    pub async fn execute(
        &self,
        action: MediaAction,
        expected_session: u64,
    ) -> Result<CommandResult, AppError> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.sender
            .try_send(Request::Action(action, expected_session, sender))
            .map_err(|_| AppError::new("busy", "The media service is busy. Please retry."))?;
        receiver
            .await
            .map_err(|_| AppError::new("stopped", "The media service has stopped."))?
    }
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.sender.try_send(Request::Stop);
        if let Ok(mut worker) = self.worker.lock()
            && let Some(worker) = worker.take()
        {
            let _ = worker.join();
        }
    }
}
impl Drop for MediaService {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn read(snapshot: &Mutex<MediaSnapshot>) -> Result<MediaSnapshot, AppError> {
    snapshot.lock().map(|s| s.clone()).map_err(|_| {
        AppError::new(
            "internal",
            "Media state is unavailable. Restart the player.",
        )
    })
}
fn publish(app: &AppHandle, shared: &Mutex<MediaSnapshot>, mut next: MediaSnapshot) {
    if let Ok(mut previous) = shared.lock()
        && !previous.same_content(&next)
    {
        next.revision = previous.revision.saturating_add(1);
        *previous = next.clone();
        drop(previous);
        if let Err(error) = app.emit("media://snapshot-updated", next) {
            diagnostics::log(&format!("Snapshot event failed: {error}"));
        }
    }
}
fn create_backend(
    stop: Arc<AtomicBool>,
    generation: Arc<AtomicU64>,
    signal: RefreshSignal,
) -> Result<Box<dyn MediaBackend>, AppError> {
    #[cfg(windows)]
    {
        windows::WindowsBackend::new(stop, generation, signal)
            .map(|b| Box::new(b) as Box<dyn MediaBackend>)
    }
    #[cfg(not(windows))]
    {
        let _ = (stop, generation, signal);
        Err(AppError::new(
            "unsupported_platform",
            "Spotify desktop media sessions require Windows 10 or 11.",
        ))
    }
}
/// Read-only, bounded native diagnostic used by the session_probe example.
pub fn diagnostic_snapshot() -> Result<MediaSnapshot, AppError> {
    diagnostic_backend()?.refresh(true)
}
pub fn diagnostic_backend() -> Result<Box<dyn MediaBackend>, AppError> {
    let (sender, _receiver) = mpsc::sync_channel(1);
    let signal = RefreshSignal {
        sender,
        dirty: Arc::new(AtomicU8::new(0)),
        count: Arc::new(AtomicU64::new(0)),
    };
    create_backend(
        Arc::new(AtomicBool::new(false)),
        Arc::new(AtomicU64::new(0)),
        signal,
    )
}
pub fn diagnostic_volume(verify: bool) -> Result<String, AppError> {
    #[cfg(windows)]
    {
        windows::probe_volume(verify)
    }
    #[cfg(not(windows))]
    {
        let _ = verify;
        Err(AppError::new(
            "unsupported_platform",
            "App volume requires Windows.",
        ))
    }
}
fn execute_confirmed(
    backend: &mut dyn MediaBackend,
    action: MediaAction,
    expected: u64,
    stop: &AtomicBool,
) -> Result<CommandResult, AppError> {
    confirm_with_timeout(backend, action, expected, stop, Duration::from_millis(1500))
}
fn confirm_with_timeout(
    backend: &mut dyn MediaBackend,
    action: MediaAction,
    expected: u64,
    stop: &AtomicBool,
    timeout: Duration,
) -> Result<CommandResult, AppError> {
    let started = Instant::now();
    let mut result = backend.execute(action, expected)?;
    loop {
        if stop.load(Ordering::Relaxed) {
            return Err(AppError::new("stopped", "The media service has stopped."));
        }
        let snapshot = backend.refresh_fast(false)?;
        let confirmed = if let MediaAction::Seek(target) = action {
            if snapshot.session_generation != expected {
                return Err(AppError::new(
                    "stale_session",
                    "Spotify changed tracks during seeking. Please retry.",
                ));
            }
            let (min, max) = (
                snapshot.seek_start_ms.unwrap_or(0),
                snapshot.seek_end_ms.or(snapshot.duration_ms).unwrap_or(0),
            );
            if max < min {
                return Err(AppError::new(
                    "invalid_seek",
                    "Spotify's seek range changed. Please retry.",
                ));
            }
            let target = target.clamp(min, max);
            let tolerance = 500
                + if snapshot.status == MediaStatus::Playing {
                    started.elapsed().as_millis().min(2000) as i64
                } else {
                    0
                };
            snapshot.position_ms.is_some_and(|position| {
                position.saturating_sub(target).saturating_abs() <= tolerance
            })
        } else {
            true
        };
        if confirmed {
            result.snapshot = Some(snapshot);
            return Ok(result);
        }
        if started.elapsed() >= timeout {
            return Err(AppError::new(
                "seek_unconfirmed",
                "Spotify has not confirmed the new position yet. Please retry.",
            ));
        }
        // A short, bounded reconciliation burst follows explicit seeks only. Normal polling stays modest.
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub fn diagnostic_command(
    backend: &mut dyn MediaBackend,
    action: MediaAction,
    expected: u64,
) -> Result<CommandResult, AppError> {
    execute_confirmed(backend, action, expected, &AtomicBool::new(false))
}
#[cfg(test)]
mod confirmation_tests {
    use super::*;
    struct Fake {
        polls: u32,
        confirm_at: u32,
        heavy_reads: u32,
    }
    impl MediaBackend for Fake {
        fn refresh(&mut self, _force: bool) -> Result<MediaSnapshot, AppError> {
            self.heavy_reads += 1;
            unreachable!("Commands must use the fast refresh path");
        }
        fn refresh_fast(&mut self, _force: bool) -> Result<MediaSnapshot, AppError> {
            self.polls += 1;
            let mut snapshot = MediaSnapshot::empty(MediaStatus::Paused);
            snapshot.source = "spotify";
            snapshot.session_generation = 7;
            snapshot.position_ms = Some(if self.polls >= self.confirm_at {
                60_000
            } else {
                10_000
            });
            snapshot.duration_ms = Some(180_000);
            snapshot.seek_start_ms = Some(0);
            snapshot.seek_end_ms = Some(180_000);
            Ok(snapshot)
        }
        fn execute(
            &mut self,
            _action: MediaAction,
            _expected: u64,
        ) -> Result<CommandResult, AppError> {
            Ok(CommandResult {
                accepted: true,
                snapshot: None,
            })
        }
    }
    #[test]
    fn seek_waits_for_authoritative_position_without_heavy_refresh_or_regular_poll_delay() {
        let mut fake = Fake {
            polls: 0,
            confirm_at: 2,
            heavy_reads: 0,
        };
        let result = confirm_with_timeout(
            &mut fake,
            MediaAction::Seek(60_000),
            7,
            &AtomicBool::new(false),
            Duration::from_millis(200),
        )
        .expect("confirmed");
        assert_eq!(result.snapshot.and_then(|s| s.position_ms), Some(60_000));
        assert_eq!(fake.polls, 2);
        assert_eq!(fake.heavy_reads, 0);
    }
    #[test]
    fn unconfirmed_seek_fails_instead_of_fabricating_target() {
        let mut fake = Fake {
            polls: 0,
            confirm_at: u32::MAX,
            heavy_reads: 0,
        };
        let error = confirm_with_timeout(
            &mut fake,
            MediaAction::Seek(60_000),
            7,
            &AtomicBool::new(false),
            Duration::ZERO,
        )
        .expect_err("not confirmed");
        assert_eq!(error.code, "seek_unconfirmed");
    }
}
