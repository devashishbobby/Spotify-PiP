# SpotifyPiP — Phase 3: Album-Reactive Glass UI, Stabilization & Windows Packaging

**Codex implementation specification**  
**Version:** 1.0 — 17 November 2026  
**Project:** The **existing** standalone `SpotifyPiP` Tauri 2 + React + TypeScript + Rust application  
**Prerequisite:** Phase 2 real Spotify/GSMTC integration should already be implemented and working, or any gaps must be documented before proceeding.  
**Read first:** `SPOTIFY_PIP_CODEX_SPEC.md` and `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md`. Inspect the *actual repository* before editing.  
**Scope:** Phase 3 only. Phase 4 is reserved for final bug polishing, documentation/demo, and GitHub publication.

---

## 1. Product goal

Turn the existing compact floating Spotify controller into an **album-reactive, cinematic glass music widget** while preserving its actual controls and interaction design. Then stabilize its native Windows lifecycle and produce a working installable desktop build.

The visual concept approved for this phase:

> **The currently playing album cover colors the atmosphere of the mini-player.** A softly blurred interpretation of the artwork sits behind a restrained frosted-glass surface, with gentle ambient waveform/glow animation while Spotify plays. Track information and existing controls remain easy to read and operate. When the song changes, the atmosphere transitions smoothly to the new cover.

A concept illustration used Kanye West's *My Beautiful Dark Twisted Fantasy* era as a **visual reference** for warm crimson album-driven ambience. This is **not** a requirement to hardcode that artist, album, track, colors, or artwork. The app must adapt to *whatever* album is playing and must use artwork provided through the existing Windows media session. Do not bundle real album art or the concept screenshot into the shipping app.

### Non-negotiable: preserve the existing UI and controls

**DO NOT redesign, relocate, resize, replace, recolor, or add to the existing buttons or interaction elements.**

This means:

- Keep the current previous/play-pause/next buttons **exactly as implemented**: appearance, colors, icons, sizes, positions, hover/focus/pressed behavior, accessibility labels, keyboard behavior, and event handlers.
- Preserve any other controls currently present, including progress/timeline, close/hide, tray, drag regions, and settings. Do not add shuffle, repeat, pin, or volume buttons just because the concept image depicts them.
- Keep current album-cover dimensions and placement, title/artist positioning, progress-bar visual style, typography, and general layout unless a minimal technical correction is essential to prevent broken rendering; report any essential change explicitly.
- Do **not** restyle the progress bar, button backgrounds, icons, or button accent colors to match the new album palette. The **background and non-interactive decorative glass/ambient layers** are the intended changes.
- Do not rewrite Phase 2 media-session code, alter established IPC contracts, change window toggle logic, or replace functional components for design convenience.
- Prefer incremental CSS layers and a small, isolated ambient-theme module. Existing controls must remain fully clickable and visually recognizable.

**Hierarchy of priorities:** Spotify-only playback correctness → preservation of existing controls → accessibility/readability → stability/performance → visual effects.

---

## 2. What Phase 3 includes

### A. Dynamic album-art-driven background

- Use the `artworkDataUrl` or equivalent **actual artwork already supplied by Phase 2** to generate a theme for the widget background.
- The app should feel visually related to the album art: e.g. a crimson/gold cover produces a warm dark crimson glow; a blue cover produces a cool navy glow.
- Render a strongly blurred, dimmed artwork background **inside the floating widget**, not across the user's desktop and not outside the window bounds.
- Derive one to three *muted* ambient colors from the album image. Apply them to the **background layer only** using CSS custom properties or a similarly contained approach.
- Make background color changes smooth when track artwork changes. Avoid flickering, flashing, jarring hue jumps, or flashes of unblurred art.
- If artwork is missing, malformed, or cannot be decoded, use the original neutral dark styling; never display a broken image.

### B. Glassmorphism treatment

- Add a restrained, dark **frosted glass** treatment to the existing panel background/underlay using opacity, subtle blur, edge highlights, and a soft shadow.
- The result should suggest tinted glass, **not** a new opaque floating card that forces a different layout.
- Ensure text, controls, title truncation, and the album image remain clear on bright, dark, saturated, and nearly monochrome covers.
- Keep glass effects within the original window footprint. Avoid changing the underlying Tauri window to fully transparent/blurred if this would introduce Windows WebView2 click-through, DPI, GPU, or rendering problems.
- Do not alter the layout or interactive elements to imitate the illustration pixel-for-pixel.

### C. Subtle playback-reactive ambient visualizer

- Add a **non-interactive** decorative visualizer *behind* the player content: faint glowing waves, understated moving light ribbons, or soft pulsing blurred shapes.
- Visualizer hues should come from the derived artwork palette, but intensity should remain subdued.
- Animate gently when the authoritative Phase 2 playback state is `playing`; pause/stop motion when `paused`, idle, disconnected, not running, or an error occurs.
- On song change, the visualizer and background should transition harmoniously to the new theme.
- Use a small, bounded number of lightweight DOM/SVG/CSS elements. Keep effects clipped inside the widget. No audio capture.
- **Technical honesty:** Windows GSMTC does not supply waveform samples or beat timestamps. This is a **playback-reactive simulated ambient animation**, **not a beat-synchronized audio analyzer**. Never claim beat-accurate syncing, fake detected BPM, or microphone/system-loopback audio access. Actual audio-reactive visualization would require a separate design and explicit permission/scope decision outside this phase.
- Honor `prefers-reduced-motion` (disable visualizer motion and crossfade movement; keep a pleasing static themed background).

### D. Windows stability and packaging (original Phase 3 scope)

- Persist and restore the window position reliably, with recovery if the monitor arrangement/DPI changes.
- Persist the existing always-on-top setting, if the app exposes it. Keep the original shortcut behavior and its known conflict handling.
- Ensure startup-hidden/tray behavior is consistent and discoverable. Preserve `Ctrl+Alt+M` and the existing tray commands.
- Prevent duplicate resident instances/windows/tray icons when possible, following supported Tauri 2 patterns.
- Ensure close/X hides the player without interrupting Spotify; tray Quit exits fully.
- Validate production startup, runtime resource loading, correct artwork handling, and packaging on the actual Windows machine.
- Create a Windows production build/installer using the existing Tauri build setup. Do not alter the app identifier unless absolutely necessary and explicitly justified.

---

## 3. Visual direction — translate the approved concept faithfully

**Mood:** dark, cinematic, warm or cool depending on the album, frosted glass, restrained ambient energy, clean and compact. The reference illustration shows a crimson-themed widget for a red album; **the implementation must be color-adaptive**.

### Visual layers, back to front

```text
┌──────────────────── existing floating window bounds ────────────────────┐
│  Layer 1  Blurred artwork / deep atmospheric gradient (clipped)         │
│  Layer 2  Low-opacity animated glow/waveform (decorative only)          │
│  Layer 3  Dark glass tint, soft border / subtle edge illumination       │
│  Layer 4  EXISTING album art, labels, controls, progress, hit targets    │
└─────────────────────────────────────────────────────────────────────────┘
```

**Layer 4 is protected.** Do not create new controls or move existing elements to match the reference image. The decorative layers must have `pointer-events: none` and must never cover or intercept a drag region, click, focus outline, tooltip, or interaction.

### Suggested visual values (starting points, not mandates)

| Element | Suggested treatment | Restriction |
| --- | --- | --- |
| Artwork backdrop | `background-size: cover`, blur roughly `28–55px`, slight scale-up so blurred edges are not exposed | Background only; clip to panel |
| Darkening overlay | Dark/black tint approximately 55–80% depending on cover luminance | Preserve text contrast |
| Ambient color | 1–3 desaturated palette colors, low opacity | Do not recolor controls |
| Glass surface | `rgba(18, 20, 24, 0.50–0.78)` or equivalent | No new nested card/layout |
| Glass edge | 1px translucent near-white border/light edge | Not bright/neon-heavy |
| Background glow | Two or three blurred, softly moving radial accents | Must remain low-key |
| Wave animation | Very slow flowing waveform or breathing gradient; optional SVG | No hard beat-sync claim |
| Song-change transition | About 350–700ms gentle crossfade | Avoid flicker; reduced motion off |

Numbers are design guidance only. Adjust for real device rendering and contrast. Prefer subtlety over a brighter animation.

### Color extraction principles

1. Obtain album art from the **existing Phase 2 image payload**; do not call Spotify Web API or external image services.
2. Use a small downsampled in-memory image (e.g. canvas 24×24 or 32×32) or a similarly bounded library to estimate the palette.
3. Prefer representative colors rather than one potentially misleading pixel; ignore near-transparent values, over-bright near-whites, and pitch-black pixels where practical.
4. Constrain saturation/luminance and darken colors before use as backgrounds. Maintain good contrast for the **unchanged** text and buttons.
5. Cache a theme based on a stable artwork/track identity, avoid computing it on every React render/animation frame, and release superseded resources.
6. Cancel or ignore stale async decoding results so quickly skipping songs cannot apply an old palette to the new song.
7. On no artwork, malformed data, decoding errors, or a disconnected session, return deterministically to a neutral dark theme. Never reuse the previous song's colors indefinitely when the media session is gone.
8. Do not store album images or private metadata persistently just for theming.

### Transition states

| Player state | Background | Visualizer |
| --- | --- | --- |
| Playing, artwork available | Album-derived tinted/blurred glass | Gentle motion |
| Paused, artwork available | Keep current album theme | Stop/freeze or settle |
| Playing, artwork unavailable | Neutral dark glass | Optional very faint neutral motion |
| Song changes | Crossfade old → new background | Smooth palette transition |
| Spotify detected but idle | Neutral dark glass | Off |
| Spotify closed/disconnected | Clear stale artwork/theme | Off |
| API error | Neutral fallback; keep error UI readable | Off |
| Reduced-motion preference | Static palette/glass | Off |

### Size and Windows framing

The original specification calls for a compact widget around **360×188 logical pixels** (subject to its actual implemented dimensions). The concept illustration is enlarged to communicate styling and is **not** permission to enlarge the app or change the control arrangement. Preserve the real window geometry and responsiveness unless a specific defect requires a documented fix.

---

## 4. Architecture and implementation boundaries

Inspect the repo and adapt to actual filenames. Favor small, replaceable presentational units; no new app scaffold.

Suggested logical additions (not mandatory paths):

```text
src/
├── hooks/useAmbientTheme.ts           # derive/update only the noninteractive theme
├── lib/ambientPalette.ts             # bounded palette extraction and fallbacks
├── components/AmbientBackdrop.tsx     # blurred art + ambient visualizer layer
└── styles/ambient-theme.css           # decorative layers; controls unchanged

src-tauri/
└── ...                                 # only Phase 3 lifecycle/packaging fixes

docs/
└── PHASE_3_WINDOWS_TESTS.md           # validation record
```

### Integration contract

- Reuse the already working media snapshot / hook / events (`media://snapshot-updated`) from Phase 2.
- The React theme feature should read **only** the fields it needs: artwork identity/data, playback state, and perhaps a track-change revision.
- Do not modify the native `MediaSnapshot` schema solely to support visual theming. If a stable identity is needed, derive it carefully in the frontend without changing existing transport logic.
- Do not add a new polling loop for audio/metadata; the Phase 2 event flow and existing display refresh are sufficient.
- Never make Rust media-session calls during rendering or animation.
- All visualizer animation stays on the client and is cheaply suspended when hidden/paused, where reliable Tauri window visibility information is available.
- Avoid new dependencies unless they clearly simplify safe, small-bundle implementation; document any added dependencies.

### Layering requirements

- Use `isolation: isolate`, contained stacking contexts, clipping/overflow, and `pointer-events: none` for decoration as appropriate.
- Preserve the current draggable zones (e.g., `data-tauri-drag-region`) and avoid overlaying clickable controls.
- Do not introduce new focusable DOM elements for the purely visual background.
- Disable text selection or pointer interactions only on decorative layers, not globally.
- Make sure transitions do not cause React component remounts or re-register media listeners.
- Do not change transport-control markup/styles or CSS selectors that control icon/size/position. Keep changes tightly scoped to the window surface/background.

### Performance guardrails

- Target smooth visuals without high CPU/GPU use in normal operation; test on the actual laptop.
- Prefer CSS transforms/opacity animations over continuously animating heavy `backdrop-filter` values.
- Avoid per-frame canvas image analysis, high-frequency JS timers, or decoding thumbnails for every state update.
- Downsample artwork once on change, cap memory and image sizes, and avoid unbounded caches.
- When hidden, do not keep expensive effects rendering needlessly.
- If blur/glass effects cause stutter, reduce layers/effects rather than sacrificing controls or reliability.
- Respect Windows scaling, light/dark accessibility settings where relevant, and reduced-motion users.

---

## 5. Phase 3 delivery order (do not skip ahead)

### Task A — Inspect and baseline the existing application

1. Read the original spec and Phase 2 spec in full.
2. Inspect current React component tree, CSS classes, media hook, artwork handling, Tauri config, tray, shortcut, positioning, and packaging status.
3. Identify which Phase 2 tests have genuinely passed and any known limitations. **Do not claim successful integration without evidence.**
4. Run baseline checks: project-defined frontend build/typecheck and Windows Rust/Tauri check as available.
5. Record/snapshot the existing buttons and layout so you can verify **zero controls regression** after styling.

**Exit condition:** Existing features and protected controls are understood; known preexisting failures are documented.

### Task B — Album theme extraction + safe fallback

1. Implement deterministic, bounded palette derivation from real artwork.
2. Generate CSS custom properties for muted/dark ambient hues.
3. Handle missing/invalid art, decode errors, fast track switching, no Spotify session, and repeated identical thumbnails.
4. Add tests for palette output ranges and safe fallbacks where practical.
5. Confirm no effect on Phase 2 media actions or their state flow.

**Exit condition:** Background themes follow actual cover art without changing controls or leaking stale colors.

### Task C — Glass background + smooth transitions

1. Add blurred artwork or background glow **behind existing elements**.
2. Add restrained frosted-glass panel effect and edge highlights.
3. Ensure no change to existing button styles, placement, label positioning, or window geometry.
4. Implement album-change crossfades and neutral fallbacks.
5. Test with vivid red, gold, blue, monochrome, and very bright sample artworks (test fixtures only, not bundled copyrighted covers).

**Exit condition:** UI feels album-reactive and glass-like while the original widget remains fully intact.

### Task D — Playback-reactive ambient visualizer

1. Add quiet, low-opacity wave/glow motion inside the widget's background layer.
2. Drive play/paused/off behavior from authoritative Phase 2 state.
3. Disable/suspend animation on reduced-motion and when the widget is hidden where possible.
4. Ensure the animation does not obstruct media controls or drag behavior.
5. Document that the visualizer is **not** actual beat detection.

**Exit condition:** Ambient movement is pleasant when playing, stationary otherwise, and cheap to render.

### Task E — Native stability and preferences

1. Save and restore position with visible-screen recovery for disconnected monitors/DPI changes.
2. Persist existing always-on-top preference and respect user choices.
3. Verify hidden-on-start, show/hide shortcut, tray menu, X-to-hide, and Quit lifecycle.
4. Add/verify a supported single-instance mechanism if compatible with existing app setup.
5. Handle restart, Spotify absent, and production startup without duplicate listeners/windows.

**Exit condition:** The Windows desktop experience is dependable and does not regress Phase 1/2 behaviors.

### Task F — Production packaging

1. Check `tauri.conf.json`, capabilities, icons, bundle settings, and app identifier. Preserve known working values.
2. Build the frontend and native release app with existing scripts; typically `npm run build`, `cargo check` (in `src-tauri`), and `npm run tauri build`, adapting to the real package scripts.
3. Locate/report actual produced `.exe`/installer artifact paths; **do not invent file paths if the build fails**.
4. Test launching the packaged app on Windows, independently of the dev terminal, with Spotify running and closed.
5. Document WebView2/Windows prerequisites and any target-machine limitations.

**Exit condition:** A release build and installable app are produced and tested where accessible; untested items are clearly labeled.

### Task G — Testing and handoff

1. Run the full Phase 3 matrix below, plus key Phase 2 regression tests.
2. Write `docs/PHASE_3_WINDOWS_TESTS.md` with pass/fail/not-verified status and specifics.
3. Update README with the visual feature's real behavior, limits, run/build commands, and installer notes.
4. Summarize changed files, added dependencies, checks executed, packaging output, known limitations, and exact next steps.
5. Keep extensive bug-polishing, final demos, and **GitHub publishing** for Phase 4, except necessary fixes to achieve Phase 3 acceptance.

---

## 6. Required acceptance criteria

Phase 3 is complete only when the following are met (or any unmet live-device item is honestly documented):

1. **All existing buttons and controls are visually and behaviorally unchanged** from the inspected pre-Phase-3 app.
2. Current Spotify album art (when exposed) determines the **background ambience**, with dark adaptive colors and blur.
3. The app shows a restrained glass treatment, without a new layout or unwanted window resizing.
4. Track change updates the ambience smoothly; rapidly skipping songs cannot leave a stale theme.
5. Playback-reactive visualizer runs only while the player is playing, except a documented neutral fallback; **not** falsely advertised as beat-accurate.
6. Pausing, closing Spotify, loss of artwork, and errors all lead to appropriate static/neutral states.
7. `prefers-reduced-motion` provides a good, stable, nonanimated experience.
8. Existing Spotify-only commands still act on Spotify when supported, even if browser media is also active.
9. `Ctrl+Alt+M`, always-on-top, dragging, close-to-hide, tray show/hide, and tray Quit still work.
10. Window position/preference persistence and offscreen recovery work or limitations are explicitly recorded.
11. No duplicate instances, subscriptions, event storms, runaway resource consumption, or notable playback/UI latency are introduced.
12. Frontend and Windows Rust/Tauri build checks complete (or errors with diagnostics are recorded).
13. A Windows production build/installer exists and is launched successfully if the machine supports it; otherwise record exactly why it could not be verified.
14. No copyrighted cover art is bundled; only user-visible, live Windows session media is used.

---

## 7. Manual Windows smoke-test matrix

Record **Pass / Fail / Not verified on this machine**, the environment, and any evidence/notes for each.

| ID | Test | Expected result |
| --- | --- | --- |
| P301 | Open player with Spotify stopped | Neutral dark glass, no stale album theme or animation |
| P302 | Play a song with a predominantly red cover | Restrained red/dark ambient background; controls unchanged |
| P303 | Switch to a blue or green cover | Correct background crossfade, not old color |
| P304 | Play a mostly white/bright cover | Strong readability; no blinding glass panel |
| P305 | Play monochrome/near-black cover | Still legible, refined neutral theme |
| P306 | Missing/corrupt/unavailable thumbnail | Stable fallback, no broken image crash |
| P307 | Pause and resume from Spotify | Ambient motion stops/resumes; transport remains authoritative |
| P308 | Pause and resume using existing button | Existing control exactly preserved; Spotify responds |
| P309 | Rapidly skip 5+ tracks | Correct final palette/art, no race/flicker/stale result |
| P310 | Play YouTube in Chrome/Edge simultaneously | Spotify remains only controlled media session |
| P311 | Toggle `Ctrl+Alt+M` while in VS Code | Show/hide reliable; no ghost visualizer/window |
| P312 | Click X, restore from tray, Quit | Original behavior preserved, proper cleanup |
| P313 | Drag player around desktop | Same drag regions and controls, no click interception |
| P314 | Change display scale or disconnect monitor | Usable window position; recover if offscreen |
| P315 | Reopen application | Preferred position/on-top setting restored |
| P316 | Launch app twice | No unwanted duplicate permanent instances/widgets/trays |
| P317 | Enable reduced-motion in OS | No ambient animation; static glass theme remains |
| P318 | Run for 20–30 minutes through track changes | No obvious memory/CPU growth or event duplication |
| P319 | Test packaged `.exe` / installer without dev terminal | Correct UI, Spotify control, images, tray, and hotkey |
| P320 | Compare pre/post screenshots and interactions | Original controls, layout, colors, labels, and sizes intact |

**Important:** Codex may not have access to your live Spotify or Windows desktop. In that case, record live scenarios as **“not verified on this machine”**; never mark them passed simply because code compiled.

### Suggested automated checks

- Palette derivation returns valid, bounded dark colors for test images and fallback for bad inputs.
- Async artwork updates ignore stale decode completion after newer songs arrive.
- Visualizer maps `playing` → animated, `paused`/idle/disconnected/error → static/off.
- No artwork → neutral theme; reappearance → current theme.
- Reduced-motion mode disables motion.
- Existing media hook / IPC contract and transport controls remain unchanged.
- Existing Phase 2 snapshot/event tests continue to pass.

---

## 8. Explicit non-goals / forbidden changes

**Do not implement in Phase 3:**

- New player buttons, new transport features, redesigned controls, changed icons, changed button colors, or a rearranged player layout.
- Shuffle/repeat/pin/volume/lyrics/playlist features shown or implied by a concept image but absent from the real UI.
- Actual beat detection, real BPM sensing, WASAPI loopback capture, microphone access, recording, or Spotify audio decoding.
- Spotify OAuth, credentials, Spotify Web API, downloads, DRM workarounds, or a third-party artwork scraping service.
- Any audio player embedded in SpotifyPiP; Spotify desktop continues playing music.
- Global desktop wallpaper changes, external waveform windows, or effects extending outside the PiP bounds.
- A new Tauri scaffold, Electron migration, backend server, unrelated project integration, or major Rust IPC rewrite.
- GitHub push, release announcement, comprehensive project showcase, or a broad post-release feature expansion; those belong in **Phase 4** unless separately requested.

If a desired effect requires changing a protected control, **skip or simplify the effect** rather than breaking the preservation rule.

---

## 9. Phase 4 boundary (for context only — do not implement now)

**Phase 4 — Final Polish, Quality Assurance & GitHub Delivery**

- Resolve remaining bugs and edge cases discovered in Phases 2/3.
- Refactor only where justified; run full regression/performance checks.
- Finish documentation, screenshots, demo material, licensing/attribution review, and usage instructions.
- Commit clean code, publish/push to GitHub as requested, and prepare a release/tag if desired.

Phase 3 must leave the project ready for this final polish, but **must not prematurely begin Phase 4**.

---

## 10. Exact Codex prompt

Paste this from the existing `SpotifyPiP` repository root, with all three specs available to Codex:

> Read `SPOTIFY_PIP_CODEX_SPEC.md`, `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md`, and `SPOTIFY_PIP_PHASE_3_CODEX_SPEC.md` fully, then inspect the existing SpotifyPiP repository. Implement **Phase 3 only**. The approved visual direction is a dark, cinematic, album-art-reactive floating widget with blurred cover-derived backgrounds, restrained glassmorphism, and a subtle playback-reactive ambient visualizer. **Do not change any existing buttons, their colors, icons, sizes, positions, labels, interactions, or the current player layout; do not add buttons shown in reference imagery.** Add only decorative, non-interactive layers beneath the existing content, with strong readability and reduced-motion support. Use the real album-art payload already available from Phase 2; never hardcode Kanye artwork or any third-party cover. The visualizer is simulated playback-state-reactive motion, **not** beat detection; do not capture system audio. Then implement the original Phase 3 Windows stability items: position/preferences persistence, monitor recovery, tray/shortcut/close lifecycle, single-instance handling where feasible, and production packaging. Preserve the existing Spotify-only GSMTC behavior, Tauri IPC, app identifier, and tray/hotkey behavior. Work through Tasks A–G in order. Run relevant frontend/Rust/Tauri checks, test on Windows where accessible, record genuinely unverified live-device behavior, create `docs/PHASE_3_WINDOWS_TESTS.md`, and report changes and build outputs. Do not stop at a plan: implement the phase. Do not start Phase 4 or push to GitHub.

---

## 11. Reference docs

- Project baseline: `SPOTIFY_PIP_CODEX_SPEC.md`.
- Phase 2 native Spotify integration: `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md`.
- Tauri 2 documentation: https://v2.tauri.app/
- Tauri 2 window state plugin: https://v2.tauri.app/plugin/window-state/
- Tauri 2 single instance plugin: https://v2.tauri.app/plugin/single-instance/
- Tauri 2 application bundling: https://v2.tauri.app/distribute/

**Final reminder:** The approved concept illustration establishes the **mood and material treatment**, not a new control layout. The original SpotifyPiP widget remains recognizably and functionally the same; only its atmosphere becomes album-reactive.
