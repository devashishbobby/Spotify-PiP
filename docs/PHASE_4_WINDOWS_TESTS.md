# Phase 4 Windows validation

Date: 17 November 2026. Author: Devashish Bobby. This phase enhances the existing application; it does not replace its approved Phase 3 background or controls.

## Baseline and preservation

Read all four specifications before implementation. Baseline passed: TypeScript, production frontend build, 17 mock/visual tests, 13 fake-IPC integration tests, Windows Rust all-target check, 17 Rust unit tests, Clippy and Rust formatting.

The exact Phase 3 backdrop component, ambient stylesheet, palette hook/extraction, player stylesheet/tokens, artwork component, controls/icons, media hook/IPC/types, native Spotify/GSMTC/volume implementation and native geometry configuration remain unchanged. Eighteen normalized source hashes are captured in `tests/fixtures/phase3-protected-sha256.json`; normalization accounts for Git's CRLF/LF conversion without accepting content changes. Three earlier exact control fixtures still pass at 320×208, 360×208 and 480×280, including hover, focus, pressed Play, volume popup and Settings.

The new anchor occupies the original cover's 88×88 space (96×96 under the existing tall-window breakpoint). The image itself is unchanged. Only the requested paused caption and live bottom-left branding text are removed; their containers retain geometry so useful footer feedback and controls do not move. The labeled development preview remains explicitly identified. App/installer/tray branding and accessible control labels remain intact.

## Implementation

- `AlbumPerimeterVisualizer.tsx` and `album-perimeter.css`: 64 thin bars, 16 per edge, with independent deterministic phases and coordinated timing on top/right/bottom/left. Bars extend outward, begin 2 px outside the cover and peak at 7 px. A 10 px clipped perimeter bounds the bars and glow. No effect is drawn over the cover interior or text/controls. Existing panel clipping adds a second boundary.
- The palette uses the existing Phase 3 hook and bounded in-memory artwork analysis/cache. Perimeter color transitions last 600 ms; neutral/missing/disconnected/error cases turn the effect off. Playing and visible enables CSS transform animation. Pause freezes the animation and fades moving pulses to a faint baseline. Hidden and reduced-motion stop animation; reduced motion retains a static perimeter.
- The equalizer and existing ambient effects are **simulated playback-reactive animation, not beat matching**. No audio samples, microphone, loopback recording, new WinRT listener or media polling were added.
- `useWindowMotion.ts` coordinates a 200 ms entrance and 140 ms exit using Web Animations API opacity plus a 4 px / 0.985 scale transform on the existing panel. Settled presentation returns to its original geometry. Reduced motion is immediate.
- Native hotkey/tray/X/Escape paths still share `window_controller` Show/Hide. A monotonic intent revision rejects old acknowledgements; only one cancellable 280 ms fallback is retained. Show exposes the native window before entrance, Hide waits for exit acknowledgement, and the fallback handles missing/reloaded/unresponsive frontend. Toggle uses intended visibility while a transition is pending, so the last request wins. Native errors roll back pending intent and restore usable presentation. No hide path sends a playback command.
- Existing Tokio gained its `time` feature for the bounded fallback. No new package dependency or frontend library was added. App identifier, shortcut/fallback, native dimensions, tray actions, on-top preference, media snapshot schema and Spotify-only matching are unchanged.

## Environment and available checks

Windows NT 10.0.26300.0 x64; Node 26.13.1; npm 11.20.0; Rust 1.100.0 MSVC. Browser tests use installed Microsoft Edge headlessly.

Computer Use was attempted using the bundled skill, then retried and reset. Each attempt returned `Computer Use native pipe is unavailable ... The system cannot find the file specified. (os error 2)`. No generic UI-automation workaround was used. Native Tauri test APIs and existing live diagnostic examples remain available; desktop appearance, mouse/hotkey feel and physical display changes are not inferred from those APIs.

| Check | Result | Evidence and scope |
| --- | --- | --- |
| `npm.cmd run check` | Pass | TypeScript |
| `npm.cmd run build` | Pass | Production assets; demo disabled in release |
| `npm.cmd test` | Pass, 17 tests | Existing visual/keyboard/seek/volume/state cases plus exact geometry and palette tests |
| `npm.cmd run test:live` | Pass, 28 tests | 13 previous cases plus 15 Phase 4 cases; fake IPC, not live Spotify |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | Pass | Native app, examples and tests |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib` | Pass, 19 tests | Previous 17 plus native visibility-intent/revision cases |
| `cargo test --manifest-path src-tauri/Cargo.toml --test native_window -- --ignored --nocapture --test-threads=1` | Pass, 1 real-window test | Hidden startup; deferred hide; stale ack after new Show; 20 rapid reversals; immediate reduced-motion hide; absent frontend fallback; actual DOM opacity restored on reduced-motion Show without a listener; position/on-top persistence |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Pass | No warnings |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Pass | Formatting |
| `npm.cmd run tauri build` | Pass | Final optimized x64 executable and NSIS installer; final build exited 0 |

The isolated native harness uses an `about:blank` WebView to test missing listeners deliberately. It has a benign missing executable icon-resource warning; production application icons remain unchanged. Native fallback/revision assertions and a DOM opacity check passed after the event loop returned. An in-progress release compile was deliberately restarted after the final native error/reduced-motion recovery fixes; the final package must include those fixes.

Browser coverage includes all four sets of 16 bars, peak travel including glow clearance, three sizes × 1/1.25/1.5 device scale, reduced motion, paused/hidden/offline states, missing art and reconnection fixtures, rapid current-palette wins, pointer pass-through and keyboard labels. Rendered-pixel comparisons show unchanged background outside the localized perimeter and unchanged artwork interior. Seven rapid frontend motion requests produce only the final acknowledgement, ignore stale events, and send no media commands. A synthetic preview was visually inspected: [Phase 4 player](validation/phase4-player.png). No real album cover was stored in fixtures or screenshots.

## Live Spotify diagnostics

The read-only session probe succeeded on the actual Windows machine. Windows enumerated one session, with exact Store source `SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify`; title/artist/artwork/timeline were available, and play/pause/next/previous/seek capabilities were enabled. This does not establish simultaneous-browser targeting, because no browser session was present.

The existing opt-in `transport_probe` verified these real Spotify operations:

| Operation | Actual observation |
| --- | --- |
| Play/pause roundtrip | Accepted and authoritative state changed; original Paused state restored; confirmation sample 21 ms |
| Seek | Two-second paused seek accepted and observed; original position restored; confirmation sample 62 ms |
| Spotify per-app volume | Accepted and observed; original level restored; confirmation sample 16 ms |
| Next | Accepted, authoritative track metadata changed; event callbacks observed; confirmation sample 21 ms |
| Previous | Accepted, authoritative track metadata changed; event callbacks observed; confirmation sample 21 ms |
| Restore paused state after skip tests | Accepted and observed; confirmation sample 21 ms |

These timings are single native command/confirmation samples, not audio-output latency or performance guarantees. Tests sent no endpoint master-volume change. They exercise the existing native backend rather than mouse-clicking the final packaged frontend. No titles, image contents or credentials were logged or committed. Spotify close/reopen and concurrent browser playback were not exercised live.

## Required Windows matrix

“Pass, automated” denotes the specific automated evidence and does not substitute for an unobserved live-device scenario.

| ID | Result | Evidence / remaining manual scope |
| --- | --- | --- |
| P401 | Pass, automated | Eighteen protected source hashes; rendered pixels outside the new perimeter and inside the original cover match. Approved backdrop/glass code and CSS unchanged. |
| P402 | Pass, automated; live observation not verified | 16 bars per edge, playing CSS animation, correct outward orientations; synthetic preview inspected. |
| P403 | Pass, automated | Exact control geometry at three sizes; maximum bar bounds avoid text, buttons and timeline. |
| P404 | Pass, automated | Peak bars/glow bounded by perimeter and panel at all tested size/DPR combinations; transparent window edges remain clipped. Native desktop edge observation still manual. |
| P405 | Pass, automated; live visual test not verified | Red/blue/gold synthetic palette and fast changes select current identity; existing live artwork retrieval confirmed by native probe. |
| P406 | Partial | Actual Spotify play/pause and restoration pass native probe. Simulated authoritative pause settles bars. Full live widget interaction not observed. |
| P407 | Not verified on this machine, live | Fake-IPC disconnect/recovery passes; actual Spotify close/reopen not performed. |
| P408 | Pass, automated | Missing/corrupt art hides perimeter and preserves honest existing fallback UI. |
| P409 | Pass, automated; live rapid skips not verified | Five-plus synthetic artwork updates converge without new media subscriptions. Live next/previous each verified separately. |
| P410 | Not verified on this machine | Real visible shortcut animation while coding needs user observation. Native registration/routing retained; frontend/native transition tests pass. |
| P411 | Not verified on this machine, full sequence | Isolated native hide/show/exit passes; real X → tray restore → tray Quit sequence not observed. |
| P412 | Pass, automated/native | Seven frontend reversals and 20 native intent toggles; stale ack protection and absent-listener fallback pass. Repeated physical keypress feel remains manual. |
| P413 | Pass, automated; live drag not verified | Pointer-events none, header reaches original native drag command, existing control handlers/hooks pass. |
| P414 | Pass | Live bottom-left label and redundant paused caption absent; existing container geometry, useful statuses, Settings and demo disclosure retained. |
| P415 | Pass, automated | Existing accessible names, focus/keyboard behavior, seek and volume controls pass regressions. |
| P416 | Pass, automated/native | Emulated media preference disables bars; native reduced-motion hide immediate; actual missing-listener Show restores DOM opacity. OS setting interaction untested. |
| P417 | Pass, emulation; physical scaling not verified | 100/125/150% device-scale contexts at all three sizes pass alignment and clipping. No physical Windows display setting was changed. |
| P418 | Not verified on this machine, live | Exact Spotify matcher and stale/non-Spotify command rejection pass units; native backend files unchanged. No concurrent YouTube session tested. |
| P419 | Not verified on this machine | No 20–30 minute visible track-change soak or GPU-jank assessment. CSS-only bounded bars and paused/hidden suspension are tested. |
| P420 | Pass, startup/deduplication; full live sequence not verified | Final release launched independently with no dev listeners and Spotify running; second launch exited 0, one resident retained. Visible UI/control sequence and installer installation remain manual. |
| P421 | Pass, staged review; push status below | No secrets/build junk; user-provided origin, `main`. No force-push, history rewrite or GitHub release. |

## Packaging

Final `npm.cmd run tauri build` exited 0. The final optimized build completed after all production source writes and includes native error/reduced-motion recovery fixes. Paths below are relative to the project root:

| Artifact | Actual path | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| App | `src-tauri/target/release/spotify-pip.exe` | 4,764,672 | `5ee1a691a80bd38697ab123ffac0479a3e782eb36b8e3e619817f3587f736cee` |
| Installer | `src-tauri/target/release/bundle/nsis/SpotifyPiP_0.1.0_x64-setup.exe` | 1,557,372 | `cf0e54ecc0a3e784a4f3aa9d2a89f3d02aa9b272fde03de1b4065f434e117fff` |

Ports 1420/1422/1423 had no listeners. `Start-Process` independently launched the final executable; PID 32428 remained alive after the startup wait, with Spotify running. A second launch exited 0 and left exactly one original resident process. A single initial native-process working set sample was 37,425,152 bytes; this excludes WebView child processes and is not a performance/soak measurement. Local ignored evidence is `docs/validation/phase4-release-launch.json`. The final release was left running for user testing.

The x64 release embeds the frontend, starts hidden and requires WebView2; it does not need Node/Vite. The NSIS installer is current-user, unsigned, and uses the existing WebView2 bootstrapper policy. Installation/uninstallation, launch with Spotify closed and clean-machine runtime setup are not verified. A successful resident-process launch does not by itself verify visible animation smoothness or all packaged controls.

## Publication and ownership

The original folder had no `.git` metadata. A local `main` repository was initialized without replacing application files. The user supplied `https://github.com/devashishbobby/Spotify-PiP` and explicitly selected MIT. README, LICENSE, npm and Cargo metadata credit **Devashish Bobby**. GitHub repository description and topics were updated successfully using existing authenticated credentials; no credential was written into source or tool output.

An initial fetch temporarily failed DNS resolution for `github.com`; subsequent fetch and remote checks succeeded and returned no branches. Final staged review excludes `node_modules`, Rust `target`, frontend `dist`, test results, caches, real artwork, credentials, local environment files, machine-specific runtime JSON and binaries. Only original icons and synthetic validation screenshots are intentional binary sources. `.env.example` is a harmless mock-mode example. Prior local release evidence JSON is explicitly ignored and is not linked as a public artifact.

Final review of 112 intended staged files found no secret/private-key/token patterns, private absolute paths, generated artifacts or unapproved environment files. `git diff --cached --check` passed; intentional Markdown hard-break whitespace is configured explicitly. The largest staged blob is Cargo.lock at about 125 KB. No remote URL was invented. No tag, force-push or GitHub release was requested or created. The implementation commit and verified push result are recorded after delivery below.
