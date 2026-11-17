//! Explicit opt-in live test. Changes Spotify playback only when invoked with an action.
use spotify_pip_lib::media_session::{
    MediaBackend, diagnostic_backend, diagnostic_command,
    state::{AppError, MediaAction, MediaSnapshot, MediaStatus},
};
use std::time::{Duration, Instant};

fn observe(
    backend: &mut dyn MediaBackend,
    before: &MediaSnapshot,
    action: MediaAction,
) -> Result<MediaSnapshot, AppError> {
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = backend.refresh(true)?;
        let changed = match action {
            MediaAction::PlayPause => snapshot.status != before.status,
            MediaAction::Seek(target) => snapshot
                .position_ms
                .is_some_and(|position| position.saturating_sub(target).abs() < 500),
            MediaAction::Volume(target) => snapshot
                .volume_level
                .is_some_and(|level| (level - target).abs() < 0.0001),
            MediaAction::Mute(target) => snapshot.volume_muted == Some(target),
            _ => snapshot.title != before.title || snapshot.artist != before.artist,
        };
        if changed {
            return Ok(snapshot);
        }
        if Instant::now() >= end {
            return Err(AppError::new(
                "not_observed",
                "Request was accepted, but an authoritative playback/track change was not observed.",
            ));
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = std::env::args().nth(1).unwrap_or_default();
    let mut action = match argument.as_str() {
        "roundtrip" | "pause" => MediaAction::PlayPause, "next" => MediaAction::Next, "previous" => MediaAction::Previous,
        "seek" => MediaAction::Seek(0),
        "volume" => MediaAction::Volume(0.0),
        _ => return Err("Choose roundtrip, pause, next, or previous. This explicit diagnostic changes Spotify playback.".into()),
    };
    let mut backend = diagnostic_backend()?;
    let before = backend.refresh(true)?;
    if argument == "volume" {
        let original = before
            .volume_level
            .ok_or("Spotify app volume unavailable")?;
        action = MediaAction::Volume(if original >= 0.5 {
            original - 0.01
        } else {
            original + 0.01
        });
    }
    if argument == "seek" {
        if before.status != MediaStatus::Paused {
            return Err("Pause Spotify before testing seek.".into());
        }
        let original = before.position_ms.ok_or("No timeline")?;
        let duration = before.duration_ms.ok_or("No duration")?;
        action = MediaAction::Seek((original + 2000).min(duration));
    }
    let events_before = backend.event_count();
    if argument == "pause" && before.status == MediaStatus::Paused {
        println!("Already paused; no command sent.");
        return Ok(());
    }
    println!(
        "Before: status={:?}, Spotify={}, generation={}",
        before.status,
        before.source == "spotify",
        before.session_generation
    );
    let started = Instant::now();
    let result = diagnostic_command(&mut *backend, action, before.session_generation)?;
    println!(
        "Command plus authoritative confirmation: {} ms",
        started.elapsed().as_millis()
    );
    println!("Request accepted={}", result.accepted);
    let after = observe(&mut *backend, &before, action)?;
    if argument == "volume" {
        let original = before.volume_level.ok_or("No original volume")?;
        diagnostic_command(
            &mut *backend,
            MediaAction::Volume(original),
            after.session_generation,
        )?;
        let restored = observe(&mut *backend, &after, MediaAction::Volume(original))?;
        println!(
            "Original Spotify app volume restored={}",
            restored
                .volume_level
                .is_some_and(|level| (level - original).abs() < 0.0001)
        );
    }
    println!(
        "Authoritative change observed: status={:?}, metadata_changed={}",
        after.status,
        after.title != before.title || after.artist != before.artist
    );
    println!(
        "Native event callbacks observed={}",
        backend.event_count().saturating_sub(events_before)
    );
    if argument == "roundtrip"
        && matches!(before.status, MediaStatus::Playing | MediaStatus::Paused)
    {
        backend.execute(MediaAction::PlayPause, after.session_generation)?;
        let restored = observe(&mut *backend, &after, MediaAction::PlayPause)?;
        println!(
            "Original playback status restored={}",
            restored.status == before.status
        );
    }
    if argument == "seek" {
        let original = before.position_ms.ok_or("No original position")?;
        backend.execute(MediaAction::Seek(original), after.session_generation)?;
        let restored = observe(&mut *backend, &after, MediaAction::Seek(original))?;
        println!(
            "Original seek position restored={}",
            restored
                .position_ms
                .is_some_and(|position| position.saturating_sub(original).abs() < 500)
        );
    }
    Ok(())
}
