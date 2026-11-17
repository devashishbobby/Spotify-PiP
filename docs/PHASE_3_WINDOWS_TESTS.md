# Phase 3 Windows validation

Implementation and checks: 17 November 2026. This report distinguishes automated evidence from live desktop tests. Phase 4 and GitHub publishing were not started.

## Environment and baseline

- Windows NT 10.0.26300.0, x64; Node 26.13.1, npm 11.20.0, Rust 1.100.0 MSVC.
- Existing Tauri 2 / React / TypeScript project, identifier `com.devas.spotifypip`, version `0.1.0`, NSIS target. Existing capabilities, dimensions, transparency, shadow setting and icons are unchanged.
- Spotify was running during native launch checks. Phase 2 observed the Store identity `SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify`. The user previously confirmed real metadata, artwork, transport and seeking. Those confirmations predate Phase 3; they are not a new visual-theme or packaged-app smoke test.
- Baseline TypeScript, production frontend build and Windows Rust check passed before changes.
- Desktop automation could not connect: `Computer Use native pipe is unavailable ... The system cannot find the file specified. (os error 2)`, including retry and runtime reset. No live desktop scenarios are inferred from compilation or headless browser results.

## Preservation evidence

The existing player stylesheet, tokens, artwork/transport/progress/volume/icon components, live media hook, IPC API/types and native media/session/volume implementation remain byte-for-byte unchanged. Baseline hashes are in [phase2-protected-sha256.json](validation/phase2-protected-sha256.json).

`PlayerWindow.tsx` imports and mounts one decorative backdrop underneath its existing content. Protected control markup, labels, SVGs, behavior, CSS and layout remain intact. Three baseline fixtures compare exact geometry and computed styles at 320×208, 360×208 and 480×280, including hover/focus/pressed Play, volume popup and Settings. All comparisons pass. Dragging and controls are not covered by a hit-testable decorative layer.

Original mock/synthetic-art screenshots only:

- [Before](validation/phase2-player.png), [Phase 3 red theme](validation/phase3-red.png).
- [Red](validation/phase3-red.png), [blue](validation/phase3-blue.png), [gold](validation/phase3-gold.png), [white](validation/phase3-white.png), [monochrome](validation/phase3-mono.png).
- [Playing with visible ambient waves](validation/phase3-playing.png).

No live copyrighted cover was saved into these fixtures or packaged resources.

## Automated checks

| Check | Result | Scope |
| --- | --- | --- |
| `npm.cmd run check` | Pass | TypeScript, no emit |
| `npm.cmd run build` | Pass | Production assets; mock provider disabled in release |
| `npm.cmd test` | Pass, 17 tests | Existing 12 visual tests, 3 control-preservation comparisons, 2 palette/race tests |
| `npm.cmd run test:live` | Pass, 13 tests | Existing 8 native-frontend hook tests plus 5 ambient tests; **fake IPC**, not live Spotify |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | Pass | Windows app, examples and tests |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | Pass, 17 tests | Existing 14 media tests plus 3 persistence/monitor tests |
| `cargo test --manifest-path src-tauri/Cargo.toml --test native_window -- --ignored --nocapture --test-threads=1` | Pass, 1 native test | Real isolated Tauri window: hidden startup, show/hide, move, on-top off, disk reload and native restoration; no Spotify commands |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Pass | No warnings |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass | Rust formatting |
| `npm.cmd run tauri build` | Pass | Windows release executable and NSIS installer produced |

The native integration harness needs Common Controls v6; its manifest is linked only into test targets in `build.rs`. Its successful run printed the restoration assertions and returned to the test runner. It also logged a missing executable icon-resource warning because the isolated test harness has no application resource icon; production icon resources were preserved. An early harness loader failure was resolved by this manifest fix.

Two initial viewport assertions reported `0.99999958` visibility for an entirely visible transformed Play button. The test now tolerates this numerical rounding and also checks all four bounds within 0.01 px. The application was not changed to satisfy the test.

## Visual behavior and resource bounds

`ambientPalette.ts` accepts bounded JPEG/PNG data URLs from the existing media snapshot. It decodes once per artwork change, validates dimensions, samples 24×24 pixels, rejects unsuitable pixels and derives muted dark colors. Missing/invalid art and non-playing/non-paused states use a neutral surface. Requests abort on replacement, stale completions are ignored, and decode has a three-second timeout.

The memory cache holds at most two tiny downsampled backgrounds, never a disk copy of live art. Crossfade layers are also bounded to two. CSS changes are scoped to decorative `.ambient-*` elements beneath the existing controls, clipped to the existing rounded window. There is no new media polling, per-frame canvas work, audio capture, microphone use, metadata IPC or transport rewrite.

The slow glow/ribbon movement is **simulated playback-state-reactive ambience, not beat detection**. It pauses when playback pauses or the native visibility state is hidden; reduced-motion removes ambient animation and transitions. Only transform/opacity animate; blur radius is static. Headless tests verify these rules. Smoothness and GPU cost on the visible laptop window still require observation.

The user checked the first release and reported that it works and the background changes dynamically, but the visualizer was not perceptible. The wave layer now sits above the artwork tint, with restrained brighter strokes and an eight-second motion cycle. A new rendered-pixel comparison verifies visible contribution in the wave band (mean channel difference >1.2 and 95th percentile >10 against decoration hidden), and a separate transform check verifies actual motion. Controls remain unchanged. The correction was visually inspected using synthetic artwork; its feel on real covers still needs user confirmation.

## Preferences and native lifecycle

`%APPDATA%\com.devas.spotifypip\window-preferences.json` stores only version, existing always-on-top choice, monitor name and work-area-relative logical position. Writes run on a debounced background worker, use temporary-file replacement and flush on normal Quit. Corrupt/oversized/unsupported settings fall back safely and surface a warning through the existing error UI. A preference-capture error cannot prevent Hide.

Restoration converts logical offsets to the current monitor scale, clamps into the work area and falls back to the primary monitor if the saved display disappears. Every Show still clamps the actual native window. No window dimensions or startup visibility are persisted. Unit tests cover changed DPI, missing monitor, offscreen offsets, invalid geometry, rapid saves, replacement and corruption. The native test verifies current-monitor position and on-top restoration. Physical display changes remain untested.

The supported [`tauri-plugin-single-instance`](https://v2.tauri.app/plugin/single-instance/) **2.5.5** is the only new runtime dependency, registered first. A second launch requests Show on the existing process. No frontend dependency was added. Original shortcut fallback, tray actions, close-to-hide and media-worker shutdown remain in place.

Development launch logged: `tray created, shortcut=Some("Ctrl+Alt+M"), visible=false, always_on_top=true`. A second debug executable exited with code 0; exactly one resident process remained with the original PID. This verifies process deduplication, not a visual tray/menu interaction.

## Required live Windows matrix

“Not verified” below means the actual user-facing scenario has not been observed in Phase 3, even when a narrower automated test passes.

| ID | Result | Evidence / remaining test |
| --- | --- | --- |
| P301 | Not verified on this machine | Headless absent/disconnected states become neutral and static; opening the packaged player with Spotify closed still needs testing. |
| P302 | Not verified on this machine | Synthetic red cover and unchanged controls pass; select a real red Spotify cover. |
| P303 | Not verified on this machine | Synthetic blue/gold changes and crossfades pass; verify live blue/green track change. |
| P304 | Not verified on this machine | Synthetic bright white cover was visually inspected and remains dark/legible. Live bright cover untested. |
| P305 | Not verified on this machine | Synthetic monochrome screenshot remains neutral/legible. Live monochrome cover untested. |
| P306 | Not verified on this machine | Corrupt/oversized/missing art fallback passes automated tests; live missing-thumbnail case not observed. |
| P307 | Not verified on this machine | Authoritative playing/paused mapping passes fake-IPC tests; external Spotify pause/resume not repeated. |
| P308 | Not verified on this machine | Existing transport and hook tests pass; user confirmed live controls in Phase 2. Repeat with Phase 3 release. |
| P309 | Not verified on this machine | Five-plus rapid synthetic updates settle on the final identity; actual rapid next-track sequence untested. |
| P310 | Not verified on this machine | Spotify identity matcher remains unchanged and fail-closed unit tests pass. Concurrent YouTube playback untested. |
| P311 | Pass, user report / partial automation | User reported the release works when asked to test Ctrl+Alt+M and dynamic background; hidden motion also stops in frontend test. Repeated long-session/focus scenarios remain untested. |
| P312 | Not verified on this machine | Hide and exit APIs pass isolated native test; full X → tray restore → tray Quit sequence needs desktop interaction. |
| P313 | Not verified on this machine | Header hit-test reaches the original native drag command; real mouse dragging not observed. |
| P314 | Not verified on this machine | DPI/missing-monitor/offscreen calculations pass units; physical scaling/disconnection test outstanding. |
| P315 | Not verified on this machine | Native move/on-top restoration and disk reload pass isolated test. Full user move/pin/Quit/relaunch sequence outstanding. |
| P316 | Pass, process behavior | Second debug and release launches exited 0 and preserved exactly one original resident process. Visible restore and tray icon count still need observation. |
| P317 | Not verified on this machine | Emulated `prefers-reduced-motion` disables animation; actual Windows accessibility setting untested. |
| P318 | Not verified on this machine | No 20–30 minute track-change soak or visible GPU/CPU assessment completed. Bounded layers/cache and listener-count tests pass. |
| P319 | Partial, full scenario not verified | Independent release startup and deduplication pass; user reports it works with dynamic real-art background. Corrected wave visibility, individual media-command sequence, tray Quit and installer wizard still need testing. |
| P320 | Pass, automated comparison | Exact pre/post control geometry/styles/icons/labels at all three sizes and existing interactions pass. Native visual comparison remains manual. |

## Release artifacts and launch

Final `npm.cmd run tauri build` completed successfully, including the frontend, optimized native executable and NSIS bundle:

| Artifact | Actual path, relative to project root | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| Release app | `src-tauri/target/release/spotify-pip.exe` | 4,732,928 | `cff4f0c2652c23599b0746e6f1d624b6da0b135848a9f79a37f82b1c606829b9` |
| NSIS installer | `src-tauri/target/release/bundle/nsis/SpotifyPiP_0.1.0_x64-setup.exe` | 1,545,870 | `85d9c6a6ad0e2c639d3fbaeb2c5ce83305441f03768fe520351d6d97ca5cac14` |

The development app was stopped before launch. Ports 1420, 1422 and 1423 had no listeners. `Start-Process` launched the final release (including the wave correction) directly, independently of npm/Vite, with Spotify running. PID 14380 remained alive after the startup wait, with an initial native-process working set of 36,982,784 bytes. This is a startup observation, not a memory/performance benchmark. See `docs/validation/phase3-release-launch.json` (local, ignored runtime evidence).

A second release process (PID 16564) exited 0 and left exactly one resident SpotifyPiP process, PID 14380. See `docs/validation/phase3-release-single-instance.json` (local, ignored runtime evidence). The release was left running for user testing. The executable was not tested with Spotify closed; the installer wizard, installation/uninstallation and clean-machine behavior were not exercised. The user's earlier release check confirms dynamic real-art backgrounds and general operation; the corrected wave treatment still needs live confirmation.

The NSIS configuration uses current-user installation and Tauri's default WebView2 bootstrapper policy. Installed WebView2 permits offline app launch; a target missing the runtime needs network access for the bootstrapper. Windows 10 version 1809 or later is required by the existing GSMTC integration. The build is x64 and locally unsigned; other architectures and a clean target Windows installation have not been tested.

## Handoff

Run `npm.cmd run tauri dev` for development. Run `npm.cmd run tauri build` to reproduce the release. Keep mock overrides off for native Spotify testing. The core source changes are the new palette/hook/backdrop/CSS, new native preferences store, lifecycle wiring and single-instance dependency. Tests, fixtures and this report cover the additions.

Remaining live checks are the matrix entries above: real covers and pause/seek/volume, concurrent browser audio, full tray/hotkey lifecycle, actual display changes, clean-machine installer prerequisites and a 20–30 minute performance soak. Phase 4 may address defects found there and prepare final delivery/GitHub work when separately requested. No OAuth, Spotify Web API, embedded playback, autostart, new controls or unrelated-project integration was added.
