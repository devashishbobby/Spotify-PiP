//! Windows per-application audio volume. No endpoint-volume setters are used.
use super::state::AppError;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE},
        Media::Audio::{Endpoints::IAudioEndpointVolume, *},
        Storage::Packaging::Appx::GetPackageFamilyName,
        System::{
            Com::{CLSCTX_ALL, CoCreateInstance},
            Threading::{
                OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
                QueryFullProcessImageNameW,
            },
        },
    },
    core::{GUID, Interface, PWSTR},
};

const CONTEXT: GUID = GUID::from_u128(0x063a33d4_e3d1_45bb_9d3b_b1d306b8c40a);
#[derive(Clone, Copy, Debug)]
pub struct VolumeState {
    pub level: f32,
    pub muted: bool,
}
struct Process(HANDLE);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

fn matches_process(family: &str, executable: &str) -> bool {
    family.eq_ignore_ascii_case("SpotifyAB.SpotifyMusic_zpdnekdrzrea0")
        && executable
            .rsplit(['\\', '/'])
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("Spotify.exe"))
}
fn spotify_process(pid: u32) -> bool {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };
    let handle = Process(handle);
    let mut family = [0u16; 128];
    let mut family_len = family.len() as u32;
    if unsafe { GetPackageFamilyName(handle.0, &mut family_len, Some(PWSTR(family.as_mut_ptr()))) }
        != ERROR_SUCCESS
        || family_len == 0
        || family_len as usize > family.len()
    {
        return false;
    }
    let mut path = [0u16; 1024];
    let mut path_len = path.len() as u32;
    if unsafe {
        QueryFullProcessImageNameW(
            handle.0,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut path_len,
        )
    }
    .is_err()
    {
        return false;
    }
    let Ok(family) = String::from_utf16(&family[..family_len.saturating_sub(1) as usize]) else {
        return false;
    };
    let Ok(path) = String::from_utf16(&path[..(path_len as usize).min(path.len())]) else {
        return false;
    };
    matches_process(&family, &path)
}
fn single_process_id(control: &IAudioSessionControl2) -> Option<u32> {
    let mut pid = 0;
    // Preserve S_OK vs AUDCLNT_S_NO_SINGLE_PROCESS; shared/system sessions fail closed.
    let result =
        unsafe { (Interface::vtable(control).GetProcessId)(Interface::as_raw(control), &mut pid) };
    (result.0 == 0 && pid != 0).then_some(pid)
}
struct VerifiedSession {
    control: IAudioSessionControl2,
    volume: ISimpleAudioVolume,
    pid: u32,
}
impl VerifiedSession {
    fn validate(&self) -> Result<(), AppError> {
        if single_process_id(&self.control) != Some(self.pid)
            || !spotify_process(self.pid)
            || !unsafe { self.control.GetState() }
                .is_ok_and(|state| state != AudioSessionStateExpired)
        {
            return Err(AppError::new(
                "stale_audio_session",
                "Spotify's audio output changed. Please retry.",
            ));
        }
        Ok(())
    }
    fn read(&self) -> Result<VolumeState, AppError> {
        self.validate()?;
        let level = unsafe { self.volume.GetMasterVolume() }
            .map_err(|e| AppError::windows("read Spotify app volume", e))?;
        let muted = unsafe { self.volume.GetMute() }
            .map_err(|e| AppError::windows("read Spotify mute state", e))?
            .as_bool();
        if !level.is_finite() || !(0.0..=1.0).contains(&level) {
            return Err(AppError::new(
                "invalid_volume",
                "Windows returned an invalid Spotify volume.",
            ));
        }
        Ok(VolumeState { level, muted })
    }
    fn level(&self, level: f32) -> Result<(), AppError> {
        self.validate()?;
        unsafe { self.volume.SetMasterVolume(level, &CONTEXT) }
            .map_err(|e| AppError::windows("change Spotify app volume", e))
    }
    fn mute(&self, muted: bool) -> Result<(), AppError> {
        self.validate()?;
        unsafe { self.volume.SetMute(muted, &CONTEXT) }
            .map_err(|e| AppError::windows("mute Spotify", e))
    }
}
struct Discovery {
    spotify: Vec<VerifiedSession>,
    other: Vec<(ISimpleAudioVolume, f32, bool)>,
    endpoints: Vec<(IAudioEndpointVolume, f32, bool)>,
}
fn discover(inspect: bool) -> Result<Discovery, AppError> {
    // Caller owns the dedicated MTA worker; all COM references are released on that thread.
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(|e| AppError::windows("enumerate audio outputs", e))?;
    let devices = unsafe { enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }
        .map_err(|e| AppError::windows("enumerate audio outputs", e))?;
    let count =
        unsafe { devices.GetCount() }.map_err(|e| AppError::windows("count audio outputs", e))?;
    if count > 64 {
        return Err(AppError::new(
            "volume_unavailable",
            "Too many audio outputs to identify Spotify volume safely.",
        ));
    }
    let mut result = Discovery {
        spotify: vec![],
        other: vec![],
        endpoints: vec![],
    };
    for index in 0..count.min(64) {
        let Ok(device) = (unsafe { devices.Item(index) }) else {
            continue;
        };
        if inspect
            && let Ok(endpoint) =
                unsafe { device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) }
            && let (Ok(level), Ok(muted)) =
                (unsafe { endpoint.GetMasterVolumeLevelScalar() }, unsafe {
                    endpoint.GetMute()
                })
        {
            result.endpoints.push((endpoint, level, muted.as_bool()));
        }
        let Ok(manager) = (unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) })
        else {
            continue;
        };
        let Ok(sessions) = (unsafe { manager.GetSessionEnumerator() }) else {
            continue;
        };
        let Ok(count) = (unsafe { sessions.GetCount() }) else {
            continue;
        };
        if !(0..=256).contains(&count) {
            return Err(AppError::new(
                "volume_unavailable",
                "Too many audio sessions to identify Spotify volume safely.",
            ));
        }
        for index in 0..count.clamp(0, 256) {
            let Ok(base) = (unsafe { sessions.GetSession(index) }) else {
                continue;
            };
            let (Ok(control), Ok(volume)) = (
                base.cast::<IAudioSessionControl2>(),
                base.cast::<ISimpleAudioVolume>(),
            ) else {
                continue;
            };
            if !unsafe { control.GetState() }.is_ok_and(|state| state != AudioSessionStateExpired) {
                continue;
            }
            let pid = single_process_id(&control);
            if let Some(pid) = pid
                && spotify_process(pid)
            {
                result.spotify.push(VerifiedSession {
                    control,
                    volume,
                    pid,
                });
            } else if inspect
                && let (Ok(level), Ok(muted)) = (unsafe { volume.GetMasterVolume() }, unsafe {
                    volume.GetMute()
                })
            {
                result.other.push((volume, level, muted.as_bool()));
            }
        }
    }
    Ok(result)
}
fn only_session(discovery: &Discovery) -> Result<&VerifiedSession, AppError> {
    if discovery.spotify.len() != 1 {
        return Err(AppError::new(
            "volume_unavailable",
            "Spotify app volume is unavailable or ambiguous. Start playback and retry.",
        ));
    }
    Ok(&discovery.spotify[0])
}
pub fn read() -> Result<Option<VolumeState>, AppError> {
    let discovery = discover(false)?;
    if discovery.spotify.len() != 1 {
        return Ok(None);
    }
    only_session(&discovery)?.read().map(Some)
}
pub fn set_level(level: f32) -> Result<(), AppError> {
    if !level.is_finite() || !(0.0..=1.0).contains(&level) {
        return Err(AppError::new(
            "invalid_volume",
            "Volume must be between 0 and 100%.",
        ));
    }
    let discovery = discover(false)?;
    only_session(&discovery)?.level(level)
}
pub fn set_muted(muted: bool) -> Result<(), AppError> {
    let discovery = discover(false)?;
    only_session(&discovery)?.mute(muted)
}
pub fn probe(verify: bool) -> Result<String, AppError> {
    let discovery = discover(true)?;
    let session = only_session(&discovery)?;
    let before = session.read()?;
    if !verify {
        return Ok(format!(
            "Verified Spotify audio sessions={}, level={:.3}, muted={}, other_sessions={}, endpoints={}",
            discovery.spotify.len(),
            before.level,
            before.muted,
            discovery.other.len(),
            discovery.endpoints.len()
        ));
    }
    let target = if before.level >= 0.5 {
        before.level - 0.01
    } else {
        before.level + 0.01
    };
    let test = (|| {
        session.level(target)?;
        let volume_verified = (session.read()?.level - target).abs() < 0.0001;
        session.mute(!before.muted)?;
        let mute_verified = session.read()?.muted != before.muted;
        Ok::<_, AppError>((volume_verified, mute_verified))
    })();
    // Always attempt restoration even if a diagnostic read or second write fails.
    let restored_level = session.level(before.level);
    let restored_mute = session.mute(before.muted);
    let (volume_verified, mute_verified) = test?;
    restored_level?;
    restored_mute?;
    let restored = session.read()?;
    let restored = (restored.level - before.level).abs() < 0.0001 && restored.muted == before.muted;
    let others_unchanged = discovery.other.iter().all(|(control, level, mute)| unsafe {
        control
            .GetMasterVolume()
            .is_ok_and(|v| (v - level).abs() < 0.0001)
            && control.GetMute().is_ok_and(|v| v.as_bool() == *mute)
    });
    let endpoints_unchanged = discovery
        .endpoints
        .iter()
        .all(|(control, level, mute)| unsafe {
            control
                .GetMasterVolumeLevelScalar()
                .is_ok_and(|v| (v - level).abs() < 0.0001)
                && control.GetMute().is_ok_and(|v| v.as_bool() == *mute)
        });
    if !volume_verified || !mute_verified || !restored || !others_unchanged || !endpoints_unchanged
    {
        return Err(AppError::new(
            "volume_not_verified",
            "Spotify-only volume verification did not pass.",
        ));
    }
    Ok(format!(
        "Volume write verified={volume_verified}; mute verified={mute_verified}; original state restored={restored}; {} other audio session(s) unchanged={others_unchanged}; {} endpoint master(s) unchanged={endpoints_unchanged}",
        discovery.other.len(),
        discovery.endpoints.len()
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_requires_exact_package_and_executable() {
        assert!(matches_process(
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0",
            r"C:\Program Files\WindowsApps\Spotify\Spotify.exe"
        ));
        assert!(!matches_process(
            "FakeSpotifyAB.SpotifyMusic_zpdnekdrzrea0",
            "Spotify.exe"
        ));
        assert!(!matches_process(
            "SpotifyAB.SpotifyMusic_zpdnekdrzrea0",
            "msedge.exe"
        ));
        assert!(!matches_process("Chrome", "Spotify.exe"));
    }
    #[test]
    fn volume_rejects_invalid_values_and_ambiguous_or_missing_sessions() {
        for level in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
            assert_eq!(
                set_level(level).expect_err("invalid volume").code,
                "invalid_volume"
            );
        }
        let discovery = Discovery {
            spotify: vec![],
            other: vec![],
            endpoints: vec![],
        };
        assert!(only_session(&discovery).is_err());
    }
}
