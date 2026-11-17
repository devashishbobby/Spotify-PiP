use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaStatus {
    Loading,
    SpotifyNotRunning,
    Idle,
    Playing,
    Paused,
    Disconnected,
    Error,
}
#[derive(Clone, Copy, Debug)]
pub enum MediaAction {
    PlayPause,
    Next,
    Previous,
    Seek(i64),
    Volume(f32),
    Mute(bool),
}
#[derive(Clone, Debug, Serialize)]
pub struct CommandResult {
    pub accepted: bool,
    pub snapshot: Option<MediaSnapshot>,
}
pub fn authorize(
    snapshot: &MediaSnapshot,
    expected: u64,
    action: MediaAction,
) -> Result<(), AppError> {
    if expected == 0
        || snapshot.source != "spotify"
        || snapshot.session_generation != expected
        || !matches!(snapshot.status, MediaStatus::Playing | MediaStatus::Paused)
    {
        return Err(AppError::new(
            "stale_session",
            "Spotify's session changed. Please retry.",
        ));
    }
    let supported = match action {
        MediaAction::PlayPause => snapshot.can_play_pause,
        MediaAction::Next => snapshot.can_next,
        MediaAction::Previous => snapshot.can_previous,
        MediaAction::Seek(_) => snapshot.can_seek,
        MediaAction::Volume(_) | MediaAction::Mute(_) => snapshot.can_set_volume,
    };
    if !supported {
        return Err(AppError::new(
            "unsupported_control",
            "Spotify does not support this control right now.",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    pub source: &'static str,
    pub status: MediaStatus,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub artwork_data_url: Option<String>,
    pub position_ms: Option<i64>,
    pub duration_ms: Option<i64>,
    pub seek_start_ms: Option<i64>,
    pub seek_end_ms: Option<i64>,
    pub last_timeline_update_utc_ms: Option<i64>,
    pub snapshot_at_utc_ms: i64,
    pub playback_rate: f64,
    pub can_play_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
    pub volume_level: Option<f32>,
    pub volume_muted: Option<bool>,
    pub can_set_volume: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub revision: u64,
    pub session_generation: u64,
}
impl MediaSnapshot {
    pub fn empty(status: MediaStatus) -> Self {
        Self {
            source: "none",
            status,
            title: None,
            artist: None,
            album: None,
            artwork_data_url: None,
            position_ms: None,
            duration_ms: None,
            seek_start_ms: None,
            seek_end_ms: None,
            last_timeline_update_utc_ms: None,
            snapshot_at_utc_ms: utc_ms(),
            playback_rate: 1.0,
            can_play_pause: false,
            can_next: false,
            can_previous: false,
            can_seek: false,
            volume_level: None,
            volume_muted: None,
            can_set_volume: false,
            error_code: None,
            error_message: None,
            revision: 0,
            session_generation: 0,
        }
    }
    pub fn failure(error: &AppError) -> Self {
        Self {
            error_code: Some(error.code.clone()),
            error_message: Some(error.message.clone()),
            ..Self::empty(MediaStatus::Error)
        }
    }
    pub fn same_content(&self, other: &Self) -> bool {
        let mut left = self.clone();
        let mut right = other.clone();
        left.snapshot_at_utc_ms = 0;
        right.snapshot_at_utc_ms = 0;
        left.revision = 0;
        right.revision = 0;
        left == right
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub hresult: Option<String>,
}
impl AppError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hresult: None,
        }
    }
    #[cfg(windows)]
    pub fn windows(operation: &str, error: windows::core::Error) -> Self {
        let hresult = format!("0x{:08X}", error.code().0 as u32);
        super::diagnostics::log(&format!("{operation} failed: {hresult}"));
        let denied = error.code().0 as u32 == 0x80070005;
        Self {
            code: if denied {
                "access_denied"
            } else {
                "native_api"
            }
            .into(),
            message: if denied {
                "Windows denied access to media sessions. Retry or check this Windows installation's media permissions.".into()
            } else {
                format!("Windows media service could not {operation}. Please retry.")
            },
            hresult: Some(hresult),
        }
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for AppError {}

pub fn utc_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
pub fn metadata(value: String) -> Option<String> {
    let value: String = value
        .chars()
        .filter(|c| !c.is_control())
        .take(1024)
        .collect();
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}
// Exact Store AUMID observed by session_probe on this machine. No substring or current-session fallback.
pub fn is_spotify_id(id: &str) -> bool {
    id.eq_ignore_ascii_case("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matcher_fails_closed() {
        assert!(is_spotify_id(
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"
        ));
        for id in [
            "chrome.exe",
            "msedge.exe",
            "Spotify",
            "Spotify.exe",
            "fake.SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify",
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify.web",
            "YouTube - Spotify",
        ] {
            assert!(!is_spotify_id(id), "{id}");
        }
    }
    #[test]
    fn controls_reject_stale_disconnected_and_unavailable_sessions() {
        let mut s = MediaSnapshot::empty(MediaStatus::Paused);
        s.source = "spotify";
        s.session_generation = 7;
        s.can_next = true;
        assert!(authorize(&s, 7, MediaAction::Next).is_ok());
        assert!(authorize(&s, 6, MediaAction::Next).is_err());
        assert!(authorize(&s, 7, MediaAction::PlayPause).is_err());
        assert!(authorize(&s, 7, MediaAction::Volume(0.5)).is_err());
        s.can_set_volume = true;
        assert!(authorize(&s, 7, MediaAction::Volume(0.5)).is_ok());
        assert!(authorize(&s, 6, MediaAction::Mute(true)).is_err());
        s.source = "none";
        assert!(authorize(&s, 7, MediaAction::Next).is_err());
        s.source = "spotify";
        s.status = MediaStatus::Disconnected;
        assert!(authorize(&s, 7, MediaAction::Next).is_err());
    }
    #[test]
    fn empty_and_errors_clear_metadata_and_controls() {
        for status in [
            MediaStatus::Idle,
            MediaStatus::SpotifyNotRunning,
            MediaStatus::Disconnected,
            MediaStatus::Error,
        ] {
            let snapshot = MediaSnapshot::empty(status);
            assert!(snapshot.title.is_none() && snapshot.position_ms.is_none());
            assert!(!snapshot.can_next && !snapshot.can_play_pause && !snapshot.can_seek);
        }
    }
    #[test]
    fn metadata_is_bounded_and_plain_text() {
        assert_eq!(metadata(" \n ".into()), None);
        assert_eq!(
            metadata("<b>Track</b>\0".into()).as_deref(),
            Some("<b>Track</b>")
        );
        assert_eq!(metadata("x".repeat(2000)).map(|s| s.len()), Some(1024));
    }
    #[test]
    fn dedup_ignores_observation_time_but_not_track_changes() {
        let a = MediaSnapshot::empty(MediaStatus::Paused);
        let mut b = a.clone();
        b.snapshot_at_utc_ms += 1;
        b.revision = 99;
        assert!(a.same_content(&b));
        b.title = Some("changed".into());
        assert!(!a.same_content(&b));
    }
}
