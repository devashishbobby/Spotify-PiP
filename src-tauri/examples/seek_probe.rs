//! Explicit, paused-only seek verification. Does not enable production seeking by itself.
#[cfg(windows)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use spotify_pip_lib::media_session::state::is_spotify_id;
    use std::time::{Duration, Instant};
    use windows::{
        Media::Control::{
            GlobalSystemMediaTransportControlsSessionManager as Manager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
        },
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    };
    if std::env::args().nth(1).as_deref() != Some("--verify") {
        return Err("Pass --verify to test a two-second seek in paused Spotify and restore the original position.".into());
    }
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED)?;
    }
    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe {
                RoUninitialize();
            }
        }
    }
    let _apartment = Apartment;
    let manager = Manager::RequestAsync()?.join()?;
    let mut selected = None;
    for session in manager.GetSessions()? {
        if is_spotify_id(&session.SourceAppUserModelId()?.to_string()) {
            selected = Some(session);
            break;
        }
    }
    let session = selected.ok_or("No verified Spotify session")?;
    let playback = session.GetPlaybackInfo()?;
    if !playback.Controls()?.IsPlaybackPositionEnabled()? {
        println!("Seeking unsupported by this Spotify session; no request sent.");
        return Ok(());
    }
    if playback.PlaybackStatus()? != Status::Paused {
        return Err("Pause Spotify before running this diagnostic.".into());
    }
    let properties = session.TryGetMediaPropertiesAsync()?.join()?;
    let title = properties.Title()?;
    let artist = properties.Artist()?;
    let timeline = session.GetTimelineProperties()?;
    let original = timeline.Position()?.Duration;
    let min = timeline
        .MinSeekTime()?
        .Duration
        .max(timeline.StartTime()?.Duration);
    let max = timeline
        .MaxSeekTime()?
        .Duration
        .min(timeline.EndTime()?.Duration);
    if max <= min {
        println!("No valid seek bounds; no request sent.");
        return Ok(());
    }
    let mut target = original.saturating_add(20_000_000).clamp(min, max);
    if target == original {
        target = original.saturating_sub(20_000_000).clamp(min, max);
    }
    let accepted = session.TryChangePlaybackPositionAsync(target)?.join()?;
    println!("Seek request accepted={accepted}");
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut observed = false;
    while Instant::now() < deadline {
        let actual = session.GetTimelineProperties()?.Position()?.Duration;
        if actual.saturating_sub(target).abs() < 5_000_000 {
            observed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let properties = session.TryGetMediaPropertiesAsync()?.join()?;
    if properties.Title()? != title || properties.Artist()? != artist {
        return Err(
            "Track changed externally; original position was not restored onto a different track."
                .into(),
        );
    }
    if !is_spotify_id(&session.SourceAppUserModelId()?.to_string()) {
        return Err("Session identity changed; restore aborted.".into());
    }
    let restored = session.TryChangePlaybackPositionAsync(original)?.join()?;
    println!(
        "Authoritative seek observed={observed}; restore request accepted={restored}; paused={}",
        session.GetPlaybackInfo()?.PlaybackStatus()? == Status::Paused
    );
    if !accepted || !observed || !restored {
        return Err("Seeking is not verified; keep production scrubbing disabled.".into());
    }
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    eprintln!("This diagnostic requires Windows.");
}
