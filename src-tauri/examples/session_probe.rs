//! Read-only diagnostic: cargo run --manifest-path src-tauri/Cargo.toml --example session_probe
#[cfg(windows)]
fn main() -> windows::core::Result<()> {
    use windows::{
        Media::Control::GlobalSystemMediaTransportControlsSessionManager as Manager,
        Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    };
    // All WinRT objects are created, used and dropped before balancing this apartment.
    unsafe {
        RoInitialize(RO_INIT_MULTITHREADED)?;
    }
    let result: windows::core::Result<()> = (|| {
        let manager = Manager::RequestAsync()?.join()?;
        println!("RequestAsync: succeeded (unpackaged Win32 diagnostic)");
        let sessions = manager.GetSessions()?;
        println!("Sessions: {}", sessions.Size()?);
        for session in sessions {
            let id = session.SourceAppUserModelId()?.to_string();
            let safe_id = if id.contains(['\\', '/']) {
                format!(
                    "<path>/{}",
                    std::path::Path::new(&id)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                )
            } else {
                id
            };
            println!(
                "Source: {safe_id:?}, playback_status={:?}",
                session.GetPlaybackInfo()?.PlaybackStatus()?
            );
        }
        Ok(())
    })();
    unsafe {
        RoUninitialize();
    }
    result?;
    let snapshot = spotify_pip_lib::media_session::diagnostic_snapshot().map_err(|error| {
        windows::core::Error::new(
            windows::core::HRESULT(0x80004005u32 as i32),
            error.to_string(),
        )
    })?;
    println!(
        "Native snapshot: status={:?}, source={}, title_available={}, artist_available={}, play_pause={}, next={}, previous={}",
        snapshot.status,
        snapshot.source,
        snapshot.title.is_some(),
        snapshot.artist.is_some(),
        snapshot.can_play_pause,
        snapshot.can_next,
        snapshot.can_previous
    );
    println!(
        "Artwork available={}, timeline position={:?}, duration={:?}, can_seek={}",
        snapshot.artwork_data_url.is_some(),
        snapshot.position_ms,
        snapshot.duration_ms,
        snapshot.can_seek
    );
    Ok(())
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows GSMTC is only available on Windows.");
}
