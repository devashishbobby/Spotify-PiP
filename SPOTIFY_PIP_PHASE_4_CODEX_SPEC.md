# SpotifyPiP — Phase 4: Album-Art Perimeter Visualizer, Motion Polish & GitHub Delivery

**Codex implementation specification**  
**Version:** 1.0 — 17 November 2026  
**Project:** The **existing** standalone `SpotifyPiP` Tauri 2 + React + TypeScript + Rust application on Windows 10/11  
**Prerequisite:** Phases 0–3 already implemented, or their unfinished items explicitly identified.  
**Read first:** `SPOTIFY_PIP_CODEX_SPEC.md`, `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md`, and `SPOTIFY_PIP_PHASE_3_CODEX_SPEC.md`. Inspect the *actual* source files before making changes.  
**Scope:** Phase 4 only. **Do not create a new application or redesign the player.**

---

## 1. Goal and the approved design

Finish SpotifyPiP with **one specific visual enhancement**, smoother window transitions, restrained text cleanup, regression testing, and a clean GitHub handoff.

**The Phase 3 glassmorphism and dynamic album-cover background are already approved. Keep them exactly as they currently look and behave.** Do **not** regenerate the background, alter its blur/opacity/color balance, change its song-transition behavior, or replace the existing ambient layer. The new visualizer is a *separate, localized* effect that hugs the **entire perimeter of the square album cover**.

The user's provided reference picture is a classic cyan equalizer with vertical bars rising and falling against a black background. Interpret its **bar rhythm and crisp rectangular bar style**, not its black backdrop or fixed cyan color. Adapt those bars into a **continuous four-sided rectangular frame** surrounding the album art, using subtle colors extracted from the current cover.

**A successful result:** The original album cover stays fully visible in its original place, with short, responsive-looking equalizer bars arranged around its **top, right, bottom, and left edges**. The glow fits entirely inside the SpotifyPiP window. The player appears/disappears more smoothly. Redundant decorative status text is gone. **Everything else remains the same.**

### Non-negotiable preservation rules

1. **Preserve the Phase 3 background and glass effect as-is.** No new full-widget visualizer, backdrop replacement, stronger blur, or desktop-spanning effects.
2. **Do not change existing player buttons** (previous, play/pause, next, and any other buttons already present): **same icons, colors, positions, sizes, hover/focus behavior, event handlers, and keyboard accessibility**.
3. Keep the current album cover image, its dimensions/position, track title/artist, progress bar, time labels, drag zones, tray, always-on-top behavior, and `Ctrl+Alt+M` shortcut.
4. The only authorized visible changes are: **(a)** add the four-sided album-cover bar visualizer; **(b)** smoother show/hide motion; **(c)** remove redundant text specified below; **(d)** necessary bug fixes that do not redesign controls.
5. No new Spotify permissions, OAuth, Web API, audio playback engine, microphone access, system audio capture, or extra backend server.
6. Spotify remains the **only** controllable media session; never fall back to browser/YouTube sessions.

**Priority order:** Working Spotify controls and original layout > contained four-sided visualizer > safe transitions > cleanup > optional visual flourish.

---

## 2. Feature A — Equalizer bars completely around the album cover

### Geometry: a closed perimeter, not a bottom equalizer

Place the animated bars **along all four sides of the actual album-art square**, rather than creating a horizontal strip under the artwork or elsewhere in the player. The surrounding bars should collectively read as **one continuous rectangular equalizer border**.

```text
              ↑ ↑ ↑ ↑ ↑ ↑ ↑ ↑ ↑ ↑
            ▏ ┌────────────────┐ ▕
            ▏ │                │ ▕
            ▏ │    EXISTING    │ ▕
            ▏ │   ALBUM ART    │ ▕
            ▏ │                │ ▕
            ▏ └────────────────┘ ▕
              ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓

  Top: bars extend upward        Bottom: bars extend downward
  Left: bars extend leftward     Right: bars extend rightward
  All four edges animate; all corners remain neatly bounded.
```

The arrows only illustrate direction; **render bars, not arrows**. Bars must not be limited to top/bottom, a circle, or a single horizontal waveform. They must not obscure the artwork, title, or controls.

### Required appearance

- Thin, tidy rectangular equalizer bars, like the uploaded reference; not a thick solid neon frame, spinning outline, generic wavy ring, or particle cloud.
- Distribute multiple small bars evenly along **each of the four sides**, with a tiny and consistent gap between adjacent bars. Corners should be tidy and not form large bright overlaps.
- Each bar has a short minimum height and varying animated amplitude. Bar length should oscillate **outward from the album edge**, with each side oriented correctly. Keep their perceived rhythm coordinated so it looks like one visualizer rather than four unrelated effects.
- **Use the current album's extracted palette already available from Phase 3.** For a red cover, use a restrained red/coral glow; for a gold cover, amber/gold; for a blue cover, cool blue. **Do not hardcode cyan or Kanye-themed colors.**
- Favor a vivid but controlled foreground edge, a very subtle glow, and enough contrast to see the bars without overpowering the cover. The Phase 3 background remains untouched.
- Keep the actual album artwork **fully visible and undistorted**. Do not lay animated bars across its interior except as a last-resort thin boundary treatment when absolutely required by the current compact layout.
- Prefer a dedicated, `aria-hidden="true"`, `pointer-events: none` visualizer wrapper positioned around the existing album-art component; do not modify transport controls to accommodate it.

### Placement and window-bound safety — mandatory

- **Nothing may spill outside the native Tauri player window.** No exterior SVG overflow, CSS overflow, box-shadow, filter/glow, expanded bounds, or escaping absolute-positioned elements.
- Provide a clipping boundary at the album visualizer layer and the top-level player content as appropriate. Use an explicit contained stacking context; e.g. `isolation`, `overflow: hidden`/clip paths, and carefully bounded blur.
- Respect the existing window's rounded corners and compact footprint; do **not** enlarge the window just to fit the effect.
- Use available whitespace around the existing artwork first. An initial target is a slim ring with ~2–4px artwork-to-bar clearance and ~4–12px maximum outward bar travel, **only if the actual layout permits it**. These values are adjustable, not absolute requirements.
- If one side has less space near an edge or another UI element, **reduce its maximum amplitude or clip it safely**. Never let it cover text, buttons, title, or progress track. Avoid arbitrary resizing/moving of the album image or controls.
- The four sides must remain visibly present even at the real mini-player size. Test on actual Windows scaling: 100%, 125%, 150% (where available).
- Do not add an extra larger glass card around the album or alter the shape/dimensions of the existing album cover.

### Animation behavior

| Spotify/player state | Four-sided album visualizer |
| --- | --- |
| Playing | Bars move with varied, tasteful amplitudes on **all four sides** |
| Paused | Bars stop smoothly and settle at a faint/static baseline; **no text label needed** |
| Track changes | Palette crossfades to the current cover without remount flicker or jumping old colors |
| No album image | Use subdued neutral color, or hide visualizer gracefully; no broken color computation |
| Spotify idle/disconnected/error | Stop; use existing empty/error UI unchanged |
| Window hidden | Suspend costly animation work when feasible |
| `prefers-reduced-motion` | Show a tasteful static perimeter or disable bars; no pulsing |

**Honest technical limitation:** Windows GSMTC provides playback/session information but **not raw audio amplitudes, beats, BPM, or waveform samples**. With the current project's scope, the moving bars can be **playback-reactive simulated equalizer animation**, not genuinely beat-synchronized. Use varied phase offsets, durations, and amplitudes to create the *look* of music-responsive motion. **Do not claim true beat matching or add system audio recording/capture without a separately approved change in scope.**

### Implementation guidance (adapt to existing source)

- Create a small `AlbumPerimeterVisualizer` presentational component or equivalent, mounted in/around the current album cover wrapper.
- A contained SVG or four small bar tracks in a positioned wrapper are both reasonable. **SVG lines/rects or CSS transforms** are preferred to a CPU-heavy per-frame canvas/React state loop. Any approach must animate all four sides.
- Reuse Phase 3 theme/palette CSS variables or the existing theme hook. Avoid new palette extraction on every animation frame.
- Each bar should animate via CSS `transform: scaleX(...)`/`scaleY(...)`, or another GPU-friendly transform that is anchored at its corresponding album edge. Respect corner spacing and the clipping rectangle.
- Keep the DOM bounded (e.g. a few dozen compact bars, not hundreds of continuously re-rendered React components) and animation logic deterministic enough for testing.
- No extra native polling, new WinRT listener, or changed media-session contract is needed. Reuse existing authoritative playback state.
- The visualizer should **never intercept a click, drag, wheel, keyboard focus, tooltip, or screen-reader action**.

---

## 3. Feature B — Smooth popup, hide and close animations

The widget's current show/hide lifecycle must remain intact, but feel more fluid.

### Desired feel

- **Show:** a short, soft fade-in plus a very small upward translate/scale settling motion, approximately **150–220ms**.
- **Hide via shortcut, close (X), or tray:** a short fade-out plus slight downward/scale motion, approximately **110–180ms**.
- Smooth and restrained; never fly across the desktop or bounce dramatically. Keep the window's actual saved position unchanged.
- `Ctrl+Alt+M` must still toggle correctly while VS Code or another application is focused.
- Hiding is **not quitting**. The background/tray/media service remains alive; music is never paused by hiding the window.

### Tauri implementation cautions

The window's native visibility and WebView animations are separate. A CSS exit animation cannot complete if the native window is hidden immediately. Therefore:

1. Inspect how the app currently toggles its native window (Rust hotkey, tray command, X/close request, or frontend bridge).
2. Funnel all hide/show entry points through **one coordinated transition mechanism** where feasible, preserving each original action.
3. On show, make the native window visible before starting the entrance animation; ensure the animation can actually render. On hide, play the short exit animation **before** native `hide()`, with a bounded fallback so it cannot get stuck.
4. Handle rapid repeated hotkey presses, consecutive show/hide requests, close during show, app shutdown, frontend reload, and a WebView that is temporarily unresponsive. **Last intended visibility wins**; no duplicate windows or permanently invisible-but-focused state.
5. Do not add unbounded timers, permanently running loops, delayed audio actions, or repeated shortcut registrations. Cancel/clean up transition timers/listeners.
6. Respect `prefers-reduced-motion`: show/hide immediately or nearly instantly without transform effects.
7. Do not change existing Tauri window sizes, topmost status, draggable regions, tray commands, saved position, or keyboard shortcut behavior solely for animation.

For a native hide triggered from Rust, choose a reliable bridge/event-and-ack mechanism or compatible fallback based on the *actual* app architecture. Avoid asserting that `opacity` on a WebView automatically animates a native Windows show/hide operation.

---

## 4. Feature C — Remove redundant visible text, and nothing else

Remove the following **only where they are unnecessary decorative/status copy**, based on inspection of the actual UI:

- The visible `Spotify PiP` / `SpotifyPiP` text in the **bottom-left** of the widget, if present. This is not the project name, window title in configuration, package ID, tray tooltip, README title, or installer branding—**do not rename the app**.
- The visible `On pause`, `Paused`, `Now paused`, or equivalent *redundant pause-state badge/caption* if it merely repeats the fact already evident from the existing play/pause icon and stopped visualizer.

**Important exceptions:**

- Preserve meaningful track title, artist, time labels, progress, connection/empty/error states (e.g. “Open Spotify to begin”, “Spotify disconnected”), and functional feedback.
- Do **not** remove any button labels used for accessibility (`aria-label`, accessible name, tooltip where useful), even if the text is no longer displayed.
- Do not remove the actual media source or state internally; Phase 2 still needs playback state and error handling.
- Remove only the specific redundant display elements and any now-unused styling/imports. Do not fill the freed space with new copy or reposition controls unless a real layout bug demands it.

---

## 5. Feature D — Final quality assurance and GitHub delivery

After the visual refinements, polish the existing app rather than adding more unrelated features:

- Fix reproducible bugs found in Phases 2–4, especially Spotify reconnect, artwork changes, frozen progress, wrong-session targeting, drag/hotkey conflicts, show/hide races, clipping, and app shutdown.
- Re-run the actual frontend typecheck/build, Rust checks, Tauri Windows packaging build, and any existing tests. Do not claim the packaged app works unless launched and tested on Windows.
- Test with Spotify playing/paused, absent, restarting; with Chrome/Edge/YouTube also playing; with rapid track changes; and with the panel shown/hidden repeatedly.
- Check resource usage and obvious animation jank over multiple tracks. Disable visualizer work while hidden/paused if feasible. Keep the app lightweight.
- Test keyboard focus, reduced motion, long titles, missing cover art, high-DPI scaling, and window boundaries.
- Update README with a short accurate feature description, usage, `Ctrl+Alt+M`, install/build instructions, and known GSMTC limitations.
- Create/update `docs/PHASE_4_WINDOWS_TESTS.md` with actual **Pass / Fail / Not verified on this machine** results; include a small description of the new perimeter visualizer and animation approach.
- Before publishing, inspect `.gitignore` and the staged diff: **do not push** `node_modules`, Rust `target`, local caches, downloaded album art, credentials, `.env` secrets, machine-specific paths, huge binaries, or transient build outputs unless intentionally distributing a release asset through the appropriate GitHub release mechanism.
- If the repo has a legitimate `origin` remote and the user has authorized credentials, make a clean commit and **push the current intended branch to GitHub** after checks. Do not force-push, rewrite history, delete branches, or invent a remote URL. If no remote/credentials/network access are available, stop short of pushing and report exact commands/actions required.
- Do not create a GitHub release/tag or publish installer assets unless that is already the user's explicit release workflow or separately requested.

---

## 6. Implementation sequence — follow in order

### Task A — Inspect and protect the current Phase 3 app

- Read all three prior specs and inspect actual source, component tree, CSS, window controller, shortcut/tray routing, existing ambient visualizer, media snapshot and current repo status.
- Capture baseline screenshots or at least inspect precise positions, typography, button styles, layout geometry, and current Phase 3 background implementation.
- Run existing build/test checks before edits and identify pre-existing issues.
- **Exit:** all protected UI areas and show/hide entry points are identified.

### Task B — Four-sided album-cover equalizer

- Implement a new contained visualizer **only around the existing album art**.
- Verify bars exist on **all four edges** and fit the current compact window.
- Reuse album palette and authoritative playing/paused state; add missing-art/reduced-motion handling.
- Keep the Phase 3 ambient background unchanged. No true beat-sync claim or audio capture.
- **Exit:** the four-sided bars are animated, album-themed, and fully within window boundaries.

### Task C — Smooth window transitions

- Implement short entrance/exit animations with safe native show/hide sequencing.
- Cover hotkey, X/hide, tray show/hide and rapid repeated toggles.
- Confirm no breaking changes to Spotify playback, hotkey registration, window position, or tray lifetime.
- **Exit:** visible, reversible animations work reliably; no stuck window.

### Task D — Redundant-text cleanup

- Remove bottom-left `Spotify PiP` visible label, and redundant visible pause-status text where they actually exist.
- Keep accessible control names, useful empty/error UI and all data fields.
- **Exit:** clean minimal presentation with no displaced controls.

### Task E — Final testing and bug fixes

- Run the matrix below, automated/unit tests where practical, frontend checks, Rust checks and Windows release build.
- Fix regressions and document legitimate unsupported Windows behaviors, not speculative passes.
- **Exit:** stable current version with accurate test record.

### Task F — Documentation and GitHub push

- Update README and `docs/PHASE_4_WINDOWS_TESTS.md`.
- Review diff for secrets/build artifacts; commit intended changes; push to the configured GitHub remote if possible.
- Summarize files changed, test results, packaged output path (if actually built), commit hash/branch/remote when verified, and anything not verified.
- **Exit:** clean documented code published when credentials/remote are available, or exact blocker identified.

---

## 7. Acceptance criteria

Phase 4 is successful when:

1. **The existing Phase 3 blurred album-art background and glass UI are visually unchanged.**
2. The **existing album cover and every control remain in their existing positions and styles**.
3. Short equalizer bars animate on **all four sides** of the cover, creating a complete perimeter; **not just below it**.
4. Bar color changes with the current album cover, without hardcoded colors or stale themes on quick track changes.
5. Bars never leak outside the **SpotifyPiP native window**, collide with controls, obscure album art, or escape rounded edges, even with glow and high DPI.
6. Bars react to authoritative playback state (move playing; stop/settle paused/offline), without false claims of real beat detection.
7. Popup/hide/close motions are visibly smooth, fast, and reliable through shortcut, X, and tray. Hiding never interrupts music.
8. The redundant bottom-left `Spotify PiP` text and decorative `On pause` text are removed if present; accessibility and useful error/status messages remain.
9. `Ctrl+Alt+M`, tray Quit, draggable window, always-on-top, Spotify-only transport actions, artwork updates, and progress all continue working.
10. Reduced motion, missing artwork, Spotify stop/restart, rapid toggles, and window visibility transitions have reasonable safe behavior.
11. The project builds/tests as supported by the machine; failures and unverified hardware cases are reported accurately.
12. Documentation is updated; the intended changes are committed and pushed if a working GitHub remote and credentials exist, otherwise publishing blockers are recorded.

---

## 8. Windows smoke-test matrix

Record **Pass / Fail / Not verified on this machine**, actual environment, and brief evidence/notes in `docs/PHASE_4_WINDOWS_TESTS.md`.

| ID | Scenario | Expected result |
| --- | --- | --- |
| P401 | Compare Phase 3 vs Phase 4 background | **Identical** backdrop/glass treatment |
| P402 | Observe four sides with a song playing | Separate short bars all around artwork, full rectangle |
| P403 | View at the actual small window size | No overlap with song text, controls or progress |
| P404 | Inspect all window edges during tall bar peaks | No bars, glow or other layers leak outside window |
| P405 | Play cover art with red then blue/gold colors | Bar palette follows current cover and transitions smoothly |
| P406 | Pause from Spotify and from existing button | Bars settle/stop; original control works |
| P407 | Close and reopen Spotify | Appropriate fallback/recovery; no stale animation |
| P408 | No artwork/invalid thumbnail | Graceful neutral/static behavior, no crash |
| P409 | Skip through five or more tracks quickly | Latest track palette wins; no accumulating animation listeners |
| P410 | Show/hide with `Ctrl+Alt+M` while coding | Entrance/exit motion smooth; hotkey remains global |
| P411 | Use X, tray Show/Hide, tray Quit | Original lifecycle preserved, clean exit |
| P412 | Rapidly toggle show/hide multiple times | No stuck hidden/visible state or duplicate windows |
| P413 | Drag and click the existing controls | No visualizer pointer interception |
| P414 | Remove bottom-left text and redundant pause label | Only requested display text is gone, no stray blank spacing |
| P415 | Check keyboard navigation/accessibility | All controls have same accessible names/focus behavior |
| P416 | Enable reduced-motion preference | No moving bars or unnecessary transition motion |
| P417 | Test at 100%, 125%, and 150% scaling where possible | Perimeter still contained and aligned |
| P418 | Play YouTube/browser audio simultaneously | Only Spotify session is controlled |
| P419 | Run through multiple tracks for 20–30 minutes | No notable CPU/memory growth or rendering stutter |
| P420 | Build and launch packaged Windows app | No dev terminal required; UI/controls/visualizer work |
| P421 | Inspect Git diff and GitHub result | Clean commit, no secrets/build junk; correct branch pushed if possible |

---

## 9. Explicit non-goals

- **Do not** redesign the background, glassmorphism, artwork, cover size/position, player layout or existing buttons.
- **Do not** add a bar visualizer that sits only on the bottom edge, a fullscreen waveform, a circular music ring, or a visualizer outside the window.
- **Do not** implement actual sound-frequency/beat extraction, WASAPI loopback, microphone recording, Spotify Web API, or unapproved audio permissions.
- **Do not** add shuffle, repeat, volume, lyrics, playlist UI, new skins or new user-facing features.
- **Do not** remove useful media/session errors, accessibility labels, title/artist/time metadata, or tray/app branding merely because decorative text is being removed.
- **Do not** alter the Rust media-session architecture, IPC, packaging ID, or global shortcut unless correcting a demonstrable bug.
- **Do not** force-push to GitHub or silently publish secrets/release assets.

---

## 10. Exact Codex prompt

Paste this from the **existing `SpotifyPiP` repository root**, with the Markdown spec files available:

> Read `SPOTIFY_PIP_CODEX_SPEC.md`, `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md`, `SPOTIFY_PIP_PHASE_3_CODEX_SPEC.md`, and `SPOTIFY_PIP_PHASE_4_CODEX_SPEC.md` in full. Inspect the existing code and implement **Phase 4 only**. The Phase 3 blurred album-cover background and glassmorphism are approved: **DO NOT change them**. Preserve every existing player button, color, icon, size, position and interaction, as well as the album cover and progress bar. Implement a **subtle rectangular equalizer made of many short bars tightly surrounding all FOUR sides of the existing album cover** (top, bottom, left, right), with bar color derived from the current album art. The visualizer must be **entirely clipped inside the SpotifyPiP window**, never obscure the album art or other UI, stop when paused, and honor reduced motion. The provided cyan bar picture is a *bar-style reference only*; do not hardcode cyan. Use lightweight playback-reactive simulated animation—GSMTC does not provide actual beats, so do not claim real audio synchronization or implement capture. Add smooth, reliable show/hide animations for the global hotkey, close/X and tray menu without breaking native window lifecycle. Remove the redundant visible `Spotify PiP` text in the widget's bottom-left and redundant visible `On pause`/`Paused` decorative label if present; preserve useful error states and all accessibility labels. Then run Windows/TypeScript/Rust/Tauri regression checks, fix actual bugs, update README and `docs/PHASE_4_WINDOWS_TESTS.md`, review the Git diff for secrets/junk, commit and push to the existing configured GitHub remote if access is available. Never force-push or invent a remote. Work through Tasks A–F rather than stopping at a plan; report all changes, test results, and genuinely unverified Windows behaviors.

---

## 11. References and implementation notes

- Original project spec: `SPOTIFY_PIP_CODEX_SPEC.md`
- Phase 2 native media integration: `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md`
- Phase 3 current approved visuals/background: `SPOTIFY_PIP_PHASE_3_CODEX_SPEC.md`
- User-provided visualizer reference: cyan/teal rectangular equalizer bars on black; use only the **bar shape/motion** idea, not the background layout or fixed color.
- Tauri documentation: https://v2.tauri.app/
- Tauri window API: https://v2.tauri.app/reference/javascript/api/namespacewindow/
- Tauri event API: https://v2.tauri.app/develop/calling-frontend/

**Final reminder:** Phase 4 enhances the album's **perimeter** and smooths the window transitions; it is **not** permission to change the beautiful Phase 3 background or redesign any existing controls.
