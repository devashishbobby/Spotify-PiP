# Phase 2 Windows validation

Results recorded on **17 November 2026**. Existing Phase 1 styling, identifier, window/tray commands and Rust global-shortcut registration were retained. No Phase 3 installer was built.

## Environment and native evidence

| Item | Observed result |
| --- | --- |
| Windows | NT 10.0.26300.0, reported by the execution host |
| Spotify installation | Microsoft Store, package version 1.309.2055.0 from the running executable path |
| Environment | Unpackaged Win32 diagnostic executable and Tauri development executable |
| Rust bindings | `windows` 0.62.2; signatures checked against installed generated bindings |
| `RequestAsync()` | Succeeded without adding a UWP capability manifest or an undocumented workaround |
| Enumerated sessions | One at probe time |
| Selected source | `SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify` |
| Playback/metadata | Paused status; real title, artist and album were present; values intentionally omitted from diagnostics |
| Controls | Play/pause, next and previous flags true; accepted requests produced authoritative OS changes |
| Artwork | Thumbnail stream decoded, passed MIME/byte/pixel limits and produced a displayable data URL |
| Timeline | Real position and duration available; noninteractive fallback covered by deterministic tests |
| Seeking | Flag and bounds valid; two-second paused seek accepted and observed; original position restored |
| Events | Metadata, playback and timeline registrations succeeded; playback/seek callbacks observed; cleanup logged on probe exit |
| Final app startup | Compiled/launched, hidden window, always-on-top enabled, tray created, `Ctrl+Alt+M` registered; real metadata/artwork/timeline initialized |
| Native UI automation | Helper's native pipe unavailable after retry/reset; UI-level scenarios remain manual |

The observed access result applies to this unpackaged development environment. It does not establish behavior on every Windows installation or in a future packaged installer. Microsoft's [`globalMediaControl` capability note](https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager) was not treated as proof of Win32 access; the executable was run and access was tested.

## Automated results

| Check | Result |
| --- | --- |
| `npm.cmd run check` | Passed |
| `npm.cmd run build` | Passed; fictional demo track metadata absent from production JS |
| `npm.cmd test` | 12 passed: visual states, mock transport/seeking, sizes, focus, reduced motion, volume popup and browser errors |
| `npm.cmd run test:live` | 8 passed with fake typed IPC and mock mode disabled; includes volume/mute IPC, pending seek feedback and failed-seek rollback |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | Passed |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | 14 passed |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Passed |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed |
| `npm.cmd run tauri dev` | Compiled and launched `spotify-pip.exe` |

Native unit tests cover exact source matching/browser rejection, stale/disconnected/unsupported command guards, empty/error mapping, bounded plain text, snapshot deduplication, 100ns conversion, nonzero starts, paused/playing anchors, invalid durations, current seek bounds and image type/dimension limits. The frontend-hook suite covers mock-free transitions, stale revisions/initial-fetch races, StrictMode listener cleanup, pending requests, no optimistic status changes, unsupported controls/timelines, mouse seek commit and hidden progress suspension.

## Manual T01–T16 matrix

**Partial** means the listed API or deterministic check passed, while the complete user-facing scenario is **not verified on this machine**. No partial result is a full manual-test pass.

| ID | Procedure and expected result | Result |
| --- | --- | --- |
| T01 | Quit Spotify, start SpotifyPiP, show it. Expect “Open Spotify to begin”, no invented song or controls. | Not verified on this machine; absent-state mapping/frontend tests passed. |
| T02 | Open Spotify and play. Expect automatic real title/artist/artwork. | Partial: native data/artwork verified and user confirmed the live widget works. Opening Spotify after app startup remains not verified on this machine. |
| T03 | Pause using Spotify's own UI. Expect widget to pause automatically. | Partial: native state changes and event callbacks observed through transport probes; external UI action not verified on this machine. |
| T04 | Toggle play/pause in the widget. Expect actual Spotify response followed by OS-derived icon state. | User-confirmed working; native transport/state-change probe also passed. |
| T05 | Use widget next/previous. Expect real metadata/track changes. | User-confirmed working; both native requests and metadata changes also passed. |
| T06 | Play a YouTube video in Chrome/Edge while Spotify plays; control the widget. Browser must remain unaffected. | Not verified on this machine; matcher rejection tests passed, and no browser/current-session fallback exists. Only one session was present during the live probe. |
| T07 | Change the song in Spotify itself. Expect new metadata/artwork/timeline. | Partial: native next/previous metadata changes verified; external UI action and image comparison not verified on this machine. |
| T08 | Close Spotify with the widget visible. Expect cleared metadata, disconnected/absent state and disabled controls. | Not verified on this machine; deterministic disconnected-state/guard tests passed. |
| T09 | Reopen Spotify after T08. Expect rediscovery and recovery. | Not verified on this machine; recovery/retry frontend tests passed. |
| T10 | Use a session without a thumbnail, or a development fake that fails reading it. Expect generic art while real metadata/controls remain. | Partial: live thumbnail success and type/dimension rejection tests passed; live missing/failed thumbnail not verified on this machine. |
| T11 | Use unsupported duration/seek data. Expect unknown labels/read-only progress. For supported seeking, compare a scrub with Spotify. | Partial: user confirmed real widget seeking, with a delay. Native fast-path confirmation now measured 62 ms and position restoration passed; post-fix interaction feel still needs a user check. Unsupported cases passed deterministic tests. |
| T12 | In VS Code, press Ctrl+Alt+M twice. Expect show/hide without changing audio. | Not verified on this machine for Phase 2; native registration succeeded. User had confirmed showing in Phase 1. |
| T13 | X/Alt+F4 hides; tray restores; pin toggle works; Quit removes process/tray/hotkey. | Not verified on this machine for Phase 2; existing handlers retained, startup and probe subscription cleanup verified. |
| T14 | Rapidly press skip/play controls. Expect bounded serialized requests and no stuck pending state/wrong target. | Partial: queue is bounded and pending/stale-context/rejection tests passed; live rapid UI interactions not verified on this machine. |
| T15 | Leave playing through multiple tracks, hidden and visible; observe CPU/memory. | Not verified on this machine; bounded artwork cache and hidden-update throttling implemented, no soak-test claim. |
| T16 | Reload the dev frontend, toggle visibility, restart Spotify. Expect one active listener set and recovery. | Partial: frontend StrictMode/race/listener tests and native cleanup passed; live Spotify restart not verified on this machine. |

Also retain the Phase 1 DPI/mixed-monitor/edge-rendering checks. The native window configuration and visual design were preserved; headless Chromium layouts do not establish Windows DPI behavior.

## Reproduce the native probes

The session probe is read-only and logs availability rather than private track values:

```powershell
$env:SPOTIFY_PIP_DIAGNOSTICS = '1'
cargo run --manifest-path src-tauri/Cargo.toml --example session_probe
```

These **explicit live diagnostics change playback** and are never invoked during normal startup:

```powershell
# Toggle, observe the OS state, then restore the original playing/paused status.
cargo run --manifest-path src-tauri/Cargo.toml --example transport_probe -- roundtrip

# Each changes the track; the probe requires an actual metadata change to report success.
cargo run --manifest-path src-tauri/Cargo.toml --example transport_probe -- next
cargo run --manifest-path src-tauri/Cargo.toml --example transport_probe -- previous

# Optional cleanup after skip testing, since Spotify may resume playback on next/previous.
cargo run --manifest-path src-tauri/Cargo.toml --example transport_probe -- pause

# Run while paused. Checks the same application adapter used by IPC and restores its old position.
cargo run --manifest-path src-tauri/Cargo.toml --example transport_probe -- seek

# Lower-level capability verification; also paused-only and restores the old position.
cargo run --manifest-path src-tauri/Cargo.toml --example seek_probe -- --verify
```

Observed transport requests returned `accepted=true`; play/pause status, next/previous metadata, and seek position were independently observed afterward. Accepted requests alone are never treated as proof of a final state. These tests did not open a browser, use Spotify credentials, capture audio, or save album artwork.

## Known limits

- Only the observed Store AUMID is allowed. Conventional desktop Spotify IDs must be captured and verified before adding exact matcher fixtures; they are not inferred from titles, process substrings or another player's session.
- JPEG/PNG thumbnails are supported. Other image formats or oversized images intentionally use the placeholder.
- No unknown timeline is fabricated; real seeking is available only with a valid timeline and control flag, and failures stay visible/recoverable.
- Installer behavior, UWP capability packaging, Windows versions other than this build, extended soak tests and complete UI smoke tests remain unverified.

## Seek responsiveness and requested volume follow-up

The user confirmed the real widget works and controls Spotify, including scrubbing, then reported delayed seek feedback and requested volume. The original post-action timeline event could be throttled until the next regular refresh, and the thumb reverted to its old position while awaiting that update.

The fix excludes artwork from command validation/confirmation, polls briefly after explicit seeks, returns the authoritative snapshot in the command response, and keeps the requested thumb position marked pending until that response. Native confirmation measured 62 ms with the original position restored afterward. Deterministic tests cover delayed confirmation, timeout without fabricated progress, pending preview retention and rollback. The latest native popup/seek feel still needs a user check; desktop UI automation remains unavailable.

Volume/mute now use the verified Spotify Windows audio session. A read-only probe found exactly one matching Spotify session. A temporary 1% volume change and mute toggle were verified and restored; three other app sessions and two endpoint master volumes were unchanged. The integrated volume command plus readback measured 16 ms, and its original level was restored. Browser tests cover popup sizing, keyboard input, mute, reopening, Escape, and exact scoped IPC arguments.

```powershell
# Read-only per-app volume discovery.
cargo run --manifest-path src-tauri/Cargo.toml --example volume_probe
# Temporary tiny volume/mute changes, restoration, and other-session/endpoint comparisons.
cargo run --manifest-path src-tauri/Cargo.toml --example volume_probe -- --verify
# The integrated command path, with original Spotify volume restored.
cargo run --manifest-path src-tauri/Cargo.toml --example transport_probe -- volume
```

The popup controls Spotify's Windows per-app mixer volume. It does not mirror Spotify's independent internal volume slider. Missing/ambiguous verified audio sessions disable volume instead of falling back to the Windows master volume.