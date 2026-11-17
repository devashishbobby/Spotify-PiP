use super::{
    MediaBackend, RefreshSignal, diagnostics,
    state::{
        AppError, CommandResult, MediaAction, MediaSnapshot, MediaStatus, authorize, is_spotify_id,
        metadata,
    },
};
use super::{
    artwork, audio_volume,
    state::utc_ms,
    timeline::{self, RawTimeline},
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use windows::{
    Foundation::TypedEventHandler,
    Media::Control::*,
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    core::RuntimeType,
};
use windows_future::{AsyncStatus, IAsyncOperation};

type Session = GlobalSystemMediaTransportControlsSession;
type Manager = GlobalSystemMediaTransportControlsSessionManager;
type PlaybackStatus = GlobalSystemMediaTransportControlsSessionPlaybackStatus;

pub(super) fn probe_volume(verify: bool) -> Result<String, AppError> {
    let _apartment = Apartment::new()?;
    super::audio_volume::probe(verify)
}

struct SessionBinding {
    session: Session,
    metadata: Option<i64>,
    playback: Option<i64>,
    timeline: Option<i64>,
}
impl SessionBinding {
    fn new(session: Session, signal: &RefreshSignal) -> Self {
        let changed = signal.clone();
        let metadata = session
            .MediaPropertiesChanged(
                &TypedEventHandler::<Session, MediaPropertiesChangedEventArgs>::new(move |_, _| {
                    changed.notify(4);
                    Ok(())
                }),
            )
            .map_err(|e| {
                diagnostics::log(&format!(
                    "Metadata event unavailable: 0x{:08X}",
                    e.code().0 as u32
                ))
            })
            .ok();
        let changed = signal.clone();
        let playback = session
            .PlaybackInfoChanged(
                &TypedEventHandler::<Session, PlaybackInfoChangedEventArgs>::new(move |_, _| {
                    changed.notify(2);
                    Ok(())
                }),
            )
            .map_err(|e| {
                diagnostics::log(&format!(
                    "Playback event unavailable: 0x{:08X}",
                    e.code().0 as u32
                ))
            })
            .ok();
        let changed = signal.clone();
        let timeline = session
            .TimelinePropertiesChanged(&TypedEventHandler::<
                Session,
                TimelinePropertiesChangedEventArgs,
            >::new(move |_, _| {
                changed.notify(1);
                Ok(())
            }))
            .map_err(|e| {
                diagnostics::log(&format!(
                    "Timeline event unavailable: 0x{:08X}",
                    e.code().0 as u32
                ))
            })
            .ok();
        diagnostics::log(&format!(
            "Spotify event subscriptions: metadata={}, playback={}, timeline={}",
            metadata.is_some(),
            playback.is_some(),
            timeline.is_some()
        ));
        Self {
            session,
            metadata,
            playback,
            timeline,
        }
    }
}
impl Drop for SessionBinding {
    fn drop(&mut self) {
        if let Some(token) = self.metadata {
            let _ = self.session.RemoveMediaPropertiesChanged(token);
        }
        if let Some(token) = self.playback {
            let _ = self.session.RemovePlaybackInfoChanged(token);
        }
        if let Some(token) = self.timeline {
            let _ = self.session.RemoveTimelinePropertiesChanged(token);
        }
        diagnostics::log("Removed Spotify session event subscriptions");
    }
}

struct Apartment;
impl Apartment {
    fn new() -> Result<Self, AppError> {
        // This dedicated worker has no pre-existing COM apartment and owns every WinRT object.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
            .map_err(|e| AppError::windows("initialize WinRT", e))?;
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            RoUninitialize();
        }
    }
}

pub(super) fn wait<T: RuntimeType>(
    operation: IAsyncOperation<T>,
    stop: &AtomicBool,
) -> Result<T, AppError> {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        if stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
            let _ = operation.Cancel();
            return Err(AppError::new(
                "timeout",
                "Windows media service did not respond. Please retry.",
            ));
        }
        let status = operation
            .Status()
            .map_err(|e| AppError::windows("read operation status", e))?;
        if status != AsyncStatus::Started {
            return operation
                .GetResults()
                .map_err(|e| AppError::windows("complete media request", e));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub struct WindowsBackend {
    manager: Manager,
    selected: Option<SessionBinding>,
    manager_token: Option<i64>,
    signal: RefreshSignal,
    generation: u64,
    next_generation: Arc<AtomicU64>,
    metadata: (Option<String>, Option<String>, Option<String>),
    metadata_at: Option<Instant>,
    artwork: Option<String>,
    artwork_at: Option<Instant>,
    last_support: Option<(bool, bool, bool, bool, bool, bool)>,
    volume: Option<audio_volume::VolumeState>,
    volume_at: Option<Instant>,
    disconnected_at: Option<Instant>,
    last_ids: Vec<String>,
    stop: Arc<AtomicBool>,
    _apartment: Apartment, // Last field: balance COM only after all WinRT references are released.
}
impl WindowsBackend {
    pub fn new(
        stop: Arc<AtomicBool>,
        next_generation: Arc<AtomicU64>,
        signal: RefreshSignal,
    ) -> Result<Self, AppError> {
        let apartment = Apartment::new()?;
        let manager = wait(
            Manager::RequestAsync()
                .map_err(|e| AppError::windows("request media-session access", e))?,
            &stop,
        )?;
        diagnostics::log("RequestAsync succeeded; unpackaged desktop process");
        let changed = signal.clone();
        let manager_token = manager
            .SessionsChanged(
                &TypedEventHandler::<Manager, SessionsChangedEventArgs>::new(move |_, _| {
                    changed.notify(4);
                    Ok(())
                }),
            )
            .map_err(|e| {
                diagnostics::log(&format!(
                    "SessionsChanged unavailable: 0x{:08X}",
                    e.code().0 as u32
                ))
            })
            .ok();
        Ok(Self {
            manager,
            manager_token,
            signal,
            selected: None,
            generation: 0,
            next_generation,
            metadata: (None, None, None),
            metadata_at: None,
            artwork: None,
            artwork_at: None,
            last_support: None,
            volume: None,
            volume_at: None,
            disconnected_at: None,
            last_ids: vec![],
            stop,
            _apartment: apartment,
        })
    }
    fn select(&mut self) -> Result<(), AppError> {
        let sessions = self
            .manager
            .GetSessions()
            .map_err(|e| AppError::windows("enumerate media sessions", e))?;
        let mut candidates = Vec::new();
        let mut ids = Vec::new();
        for session in sessions {
            let Ok(id) = session.SourceAppUserModelId() else {
                continue;
            };
            let id = id.to_string();
            ids.push(diagnostics::safe_id(&id));
            if is_spotify_id(&id) {
                let playing = session
                    .GetPlaybackInfo()
                    .and_then(|p| p.PlaybackStatus())
                    .is_ok_and(|s| s == PlaybackStatus::Playing);
                let retained = self.selected.as_ref().map(|s| &s.session) == Some(&session);
                candidates.push((playing, retained, session));
            }
        }
        if ids != self.last_ids {
            diagnostics::log(&format!("Enumerated {} session(s): {ids:?}", ids.len()));
            self.last_ids = ids;
        }
        // Playing first, then the retained Spotify session, then enumeration order. Never a browser.
        candidates.sort_by_key(|(playing, retained, _)| (!*playing, !*retained));
        let selected = candidates.into_iter().next().map(|(_, _, session)| session);
        if selected.as_ref() != self.selected.as_ref().map(|s| &s.session) {
            diagnostics::log(if selected.is_some() {
                "Selected verified Spotify Store session"
            } else {
                "Spotify session disconnected"
            });
            self.generation = self
                .next_generation
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            self.disconnected_at = self
                .selected
                .as_ref()
                .filter(|_| selected.is_none())
                .map(|_| Instant::now());
            self.selected = selected.map(|session| SessionBinding::new(session, &self.signal));
            self.metadata = (None, None, None);
            self.metadata_at = None;
            self.artwork = None;
            self.artwork_at = None;
            self.last_support = None;
            self.volume = None;
            self.volume_at = None;
        }
        Ok(())
    }
}
impl MediaBackend for WindowsBackend {
    fn event_count(&self) -> u64 {
        self.signal.count.load(Ordering::Relaxed)
    }
    fn execute(
        &mut self,
        action: MediaAction,
        expected_session: u64,
    ) -> Result<CommandResult, AppError> {
        let snapshot = self.read_snapshot(true, false)?; // Validate context without waiting for artwork.
        authorize(&snapshot, expected_session, action)?;
        let session = &self
            .selected
            .as_ref()
            .ok_or_else(|| AppError::new("no_session", "Spotify disconnected."))?
            .session;
        let id = session
            .SourceAppUserModelId()
            .map_err(|e| AppError::windows("validate Spotify session", e))?;
        if !is_spotify_id(&id.to_string()) {
            return Err(AppError::new(
                "wrong_source",
                "The selected session is not Spotify.",
            ));
        }
        match action {
            MediaAction::Volume(level) => {
                audio_volume::set_level(level)?;
                self.volume = audio_volume::read()?;
                self.volume_at = Some(Instant::now());
                return Ok(CommandResult {
                    accepted: true,
                    snapshot: None,
                });
            }
            MediaAction::Mute(muted) => {
                audio_volume::set_muted(muted)?;
                self.volume = audio_volume::read()?;
                self.volume_at = Some(Instant::now());
                return Ok(CommandResult {
                    accepted: true,
                    snapshot: None,
                });
            }
            _ => {}
        }
        let operation = match action {
            MediaAction::PlayPause => session.TryTogglePlayPauseAsync(),
            MediaAction::Next => session.TrySkipNextAsync(),
            MediaAction::Previous => session.TrySkipPreviousAsync(),
            MediaAction::Seek(position) => {
                let time = session
                    .GetTimelineProperties()
                    .map_err(|e| AppError::windows("validate seek timeline", e))?;
                let start = time
                    .StartTime()
                    .map_err(|e| AppError::windows("validate seek start", e))?
                    .Duration;
                let end = time
                    .EndTime()
                    .map_err(|e| AppError::windows("validate seek end", e))?
                    .Duration;
                let min = time
                    .MinSeekTime()
                    .map_err(|e| AppError::windows("validate minimum seek", e))?
                    .Duration;
                let max = time
                    .MaxSeekTime()
                    .map_err(|e| AppError::windows("validate maximum seek", e))?
                    .Duration;
                let ticks =
                    timeline::seek_ticks(position, start, end, min, max).ok_or_else(|| {
                        AppError::new(
                            "invalid_seek",
                            "The requested position is outside Spotify's timeline.",
                        )
                    })?;
                session.TryChangePlaybackPositionAsync(ticks)
            }
            MediaAction::Volume(_) | MediaAction::Mute(_) => {
                return Err(AppError::new(
                    "invalid_control",
                    "This audio control could not be dispatched.",
                ));
            }
        }
        .map_err(|e| AppError::windows("request Spotify playback control", e))?;
        if !wait(operation, &self.stop)? {
            return Err(AppError::new(
                "request_rejected",
                "Spotify declined this playback request. Please retry.",
            ));
        }
        Ok(CommandResult {
            accepted: true,
            snapshot: None,
        })
    }
    fn refresh(&mut self, force_metadata: bool) -> Result<MediaSnapshot, AppError> {
        self.read_snapshot(force_metadata, true)
    }
    fn refresh_fast(&mut self, force_metadata: bool) -> Result<MediaSnapshot, AppError> {
        self.read_snapshot(force_metadata, false)
    }
}
impl WindowsBackend {
    fn read_snapshot(
        &mut self,
        force_metadata: bool,
        load_artwork: bool,
    ) -> Result<MediaSnapshot, AppError> {
        self.select()?;
        let Some(binding) = self.selected.as_ref() else {
            let status = if self
                .disconnected_at
                .is_some_and(|at| at.elapsed() < Duration::from_secs(6))
            {
                MediaStatus::Disconnected
            } else {
                MediaStatus::SpotifyNotRunning
            };
            return Ok(MediaSnapshot::empty(status));
        };
        let session = &binding.session;
        if force_metadata
            || self
                .metadata_at
                .is_none_or(|at| at.elapsed() > Duration::from_secs(5))
        {
            let properties = wait(
                session
                    .TryGetMediaPropertiesAsync()
                    .map_err(|e| AppError::windows("read Spotify metadata", e))?,
                &self.stop,
            )?;
            let next_metadata = (
                properties
                    .Title()
                    .ok()
                    .and_then(|s| metadata(s.to_string())),
                properties
                    .Artist()
                    .ok()
                    .and_then(|s| metadata(s.to_string())),
                properties
                    .AlbumTitle()
                    .ok()
                    .and_then(|s| metadata(s.to_string())),
            );
            let changed = self.metadata != next_metadata;
            if changed {
                self.artwork = None;
                self.artwork_at = None;
                if self.metadata_at.is_some() {
                    self.generation = self
                        .next_generation
                        .fetch_add(1, Ordering::Relaxed)
                        .saturating_add(1);
                }
                diagnostics::log(&format!(
                    "Spotify metadata changed: title={}, artist={}, album={}",
                    next_metadata.0.is_some(),
                    next_metadata.1.is_some(),
                    next_metadata.2.is_some()
                ));
            }
            if load_artwork
                && (changed
                    || self.artwork.is_none()
                        && self
                            .artwork_at
                            .is_none_or(|at| at.elapsed() > Duration::from_secs(15)))
            {
                self.artwork =
                    properties.Thumbnail().ok().and_then(|reference| {
                        match artwork::read(&reference, &self.stop) {
                            Ok(image) => Some(image),
                            Err(error) => {
                                diagnostics::log(&format!("Artwork fallback: {}", error.code));
                                None
                            }
                        }
                    });
                self.artwork_at = Some(Instant::now());
            }
            self.metadata = next_metadata;
            self.metadata_at = Some(Instant::now());
        }
        let playback = session
            .GetPlaybackInfo()
            .map_err(|e| AppError::windows("read Spotify playback state", e))?;
        let status = playback
            .PlaybackStatus()
            .map_err(|e| AppError::windows("read playback status", e))?;
        let status = if self.metadata.0.is_none() {
            MediaStatus::Idle
        } else if status == PlaybackStatus::Playing {
            MediaStatus::Playing
        } else if status == PlaybackStatus::Paused {
            MediaStatus::Paused
        } else {
            MediaStatus::Idle
        };
        let active = matches!(status, MediaStatus::Playing | MediaStatus::Paused);
        if load_artwork
            && (force_metadata
                || self
                    .volume_at
                    .is_none_or(|at| at.elapsed() >= Duration::from_secs(2)))
        {
            let before = self.volume.is_some();
            self.volume = match audio_volume::read() {
                Ok(volume) => volume,
                Err(error) => {
                    if before || self.volume_at.is_none() {
                        diagnostics::log(&format!("App volume unavailable: {}", error.code));
                    }
                    None
                }
            };
            self.volume_at = Some(Instant::now());
            if before != self.volume.is_some() {
                diagnostics::log(&format!(
                    "Verified Spotify app volume available={}",
                    self.volume.is_some()
                ));
            }
        }
        let controls = playback.Controls().ok();
        let now = utc_ms();
        let rate = playback
            .PlaybackRate()
            .and_then(|rate| rate.Value())
            .ok()
            .filter(|rate| rate.is_finite() && *rate >= 0.0 && *rate <= 4.0)
            .unwrap_or(1.0);
        let timeline = if active {
            session.GetTimelineProperties().ok().and_then(|time| {
                timeline::normalize(
                    RawTimeline {
                        start: time.StartTime().ok()?.Duration,
                        end: time.EndTime().ok()?.Duration,
                        position: time.Position().ok()?.Duration,
                        updated: time
                            .LastUpdatedTime()
                            .map(|time| time.UniversalTime)
                            .unwrap_or(0),
                        min_seek: time.MinSeekTime().map(|t| t.Duration).unwrap_or(0),
                        max_seek: time.MaxSeekTime().map(|t| t.Duration).unwrap_or(0),
                    },
                    status == MediaStatus::Playing,
                    rate,
                    now,
                )
            })
        } else {
            None
        };
        let mut snapshot = MediaSnapshot {
            source: "spotify",
            status,
            title: self.metadata.0.clone(),
            artist: self.metadata.1.clone(),
            album: self.metadata.2.clone(),
            artwork_data_url: self.artwork.clone(),
            volume_level: self.volume.map(|volume| volume.level),
            volume_muted: self.volume.map(|volume| volume.muted),
            can_set_volume: active && self.volume.is_some(),
            can_play_pause: active
                && controls
                    .as_ref()
                    .is_some_and(|c| c.IsPlayPauseToggleEnabled().unwrap_or(false)),
            can_next: active
                && controls
                    .as_ref()
                    .is_some_and(|c| c.IsNextEnabled().unwrap_or(false)),
            can_previous: active
                && controls
                    .as_ref()
                    .is_some_and(|c| c.IsPreviousEnabled().unwrap_or(false)),
            position_ms: timeline.as_ref().map(|t| t.position_ms),
            duration_ms: timeline.as_ref().map(|t| t.duration_ms),
            seek_start_ms: timeline
                .as_ref()
                .and_then(|t| t.seek_bounds.map(|bounds| bounds.0)),
            seek_end_ms: timeline
                .as_ref()
                .and_then(|t| t.seek_bounds.map(|bounds| bounds.1)),
            last_timeline_update_utc_ms: timeline.as_ref().and_then(|t| t.last_updated_utc_ms),
            playback_rate: rate,
            snapshot_at_utc_ms: now,
            session_generation: self.generation,
            ..MediaSnapshot::empty(status)
        };
        let seek_flag = controls
            .as_ref()
            .is_some_and(|c| c.IsPlaybackPositionEnabled().unwrap_or(false));
        // Verified with the exact Store AUMID on this machine by seek_probe on 2026-11-17.
        // Availability is still checked on every snapshot and every request may be refused by Windows.
        snapshot.can_seek =
            active && seek_flag && timeline.as_ref().is_some_and(|t| t.seek_bounds.is_some());
        let support = (
            snapshot.can_play_pause,
            snapshot.can_next,
            snapshot.can_previous,
            self.artwork.is_some(),
            timeline.is_some(),
            seek_flag,
        );
        if self.last_support != Some(support) {
            diagnostics::log(&format!(
                "Spotify support: play_pause={}, next={}, previous={}, artwork={}, timeline={}, seek_flag={}, valid_seek_bounds={}",
                support.0,
                support.1,
                support.2,
                support.3,
                support.4,
                support.5,
                timeline.as_ref().is_some_and(|t| t.seek_bounds.is_some())
            ));
            self.last_support = Some(support);
        }
        Ok(snapshot)
    }
}
impl Drop for WindowsBackend {
    fn drop(&mut self) {
        self.selected = None;
        if let Some(token) = self.manager_token.take() {
            let _ = self.manager.RemoveSessionsChanged(token);
        }
        diagnostics::log("Removed manager subscription; releasing WinRT service");
    }
}
