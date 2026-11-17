# Spotify PiP — Standalone Windows Floating Mini Player

**Implementation specification for Codex**  
**Version:** 1.0 — MVP first  
**Target:** Windows 10/11 desktop  
**Project location:** A **new, independent folder** named `SpotifyPiP`. **Do not modify JAM or LocalCompanion.**

---

## 1. Product brief

Build a polished, lightweight, always-on-top **Picture-in-Picture-style controller for the installed Spotify desktop app** on Windows.

Spotify itself continues to play the audio. Our application shows a small floating music-control window that can be summoned or hidden using a **global keyboard shortcut**, even while the user is coding or using another app.

This is **not** a Spotify clone, a streaming client, or an embedded Spotify webpage. It is a separate native Windows utility that observes and controls Spotify's Windows media session.

**Main user story:**

> I'm coding with Spotify playing in the background. I press a keyboard shortcut, and a beautiful, compact floating player appears above my editor. I can see the album art and song details, pause, skip tracks, and hide it with the same shortcut without switching to the full Spotify app.

### MVP definition of done

- Windows desktop `.exe` launches successfully.
- Global hotkey toggles the floating player, even if the player window is hidden.
- Spotify desktop session is detected without opening a browser or requiring Spotify credentials.
- Track title, artist, artwork (when exposed), and playback state appear accurately.
- Play/pause, previous, and next work against the correct Spotify session, when supported.
- A compact, visually refined, always-on-top, draggable floating window works on Windows.
- Closing the floating window **hides** it; the app remains available from a system tray icon. Tray **Quit** fully exits.
- All missing-media and permissions/registration failures have clear, useful UI states; no invented metadata or fake controls.
- A release build works on the developer's Windows machine.

---

## 2. Technical stack

| Layer | Technology | Responsibility |
| --- | --- | --- |
| Desktop framework | **Tauri 2** | Native app window, tray, global shortcut, packaging |
| Frontend | **React + TypeScript + Vite** | UI, state, animations, accessibility |
| Styling | CSS Modules or well-organized plain CSS | High-quality lightweight visual design |
| Native backend | **Rust** | Windows media-session integration and playback commands |
| OS integration | **Windows `Windows.Media.Control` / GSMTC APIs** | Discover Spotify media session; read metadata and control playback |
| Icons | Lucide React (optional) | Consistent, minimal line icons |
| Persistence | Small local settings file / Tauri store plugin (only if needed) | Shortcut preference, always-on-top state, window preference |

**Do not introduce** Electron, a Node.js server, a Python service, a Spotify Web API dependency, Spotify OAuth, a database, or unrelated JAM code. Avoid unnecessary dependencies.

### Windows media API strategy

Use `GlobalSystemMediaTransportControlsSessionManager` from the Windows `Windows.Media.Control` namespace via Rust's `windows` crate or another well-maintained native interop path.

- Call `RequestAsync` to get the media-session manager.
- Enumerate sessions using `GetSessions()`.
- Select the **Spotify session**, identified reliably by `SourceAppUserModelId` or other verified Spotify-specific metadata. **Do not blindly control `GetCurrentSession()`**, because it may refer to a browser or another player.
- Read media metadata using `TryGetMediaPropertiesAsync()`.
- Read playback status using `GetPlaybackInfo()`.
- Read timeline properties using `GetTimelineProperties()` when exposed.
- Request transport actions using `TryTogglePlayPauseAsync()`, `TrySkipPreviousAsync()`, `TrySkipNextAsync()`.
- If supported, use the corresponding seek API for a real, interactive timeline. Treat support as conditional.
- Observe session, media-properties, playback-info and timeline changes. Where subscriptions are difficult or fragile, use a modest, well-managed refresh/polling fallback. Clean up subscriptions and timers.
- Read thumbnail/artwork streams from Windows if available; convert safely into displayable bytes or a data URL with size/type safeguards. If absent, show a designed placeholder.
- Never claim that unsupported actions succeeded. Handle false returns and errors.

**Important compatibility note:** Some Windows APIs document an app capability named `globalMediaControl`. Determine whether the chosen Win32/Tauri packaging and API path can access GSMTC directly, and validate this on the actual Windows machine. Do not silently assume packaged-UWP behavior equals unpackaged Win32 behavior.

---

## 3. Interaction design

### Default launch behavior

1. App starts and creates a tray icon.
2. Register the global hotkey, default **`Ctrl+Alt+M`** (make configurable later).
3. Start the native Spotify media-session listener.
4. Keep the floating player **hidden at startup by default**; show a one-time onboarding hint/notification or tray hint only if necessary, without spamming.
5. Pressing the shortcut toggles the window **show ↔ hide**.
6. When shown, focus the window and retain always-on-top behavior (without stealing focus repeatedly when metadata updates).
7. Closing with **X** hides the floating window but leaves the tray/shortcut/background controller running. Tray **Quit** exits cleanly.

### Tray menu

- **Show / Hide Player**
- **Always on top** (checked state; default on)
- **Settings** (can be minimal)
- **Quit**

Double-clicking the tray icon may toggle the player when supported. Do not rely on double-click as the only access method.

### Global shortcut

Use Tauri 2's **Global Shortcut plugin**. Prefer Rust-side registration/handling so the hotkey remains active while the WebView is hidden.

- Default: `Ctrl+Alt+M`.
- On conflict/registration failure, show a clear message accessible from the tray/settings; provide a fallback shortcut or user-selectable binding.
- Avoid registering the shortcut twice, particularly during hot reload.
- Unregister or dispose when changing bindings or quitting.
- Tauri capability permissions must be configured appropriately if any shortcut API is invoked from frontend JS.
- A click on the tray's Show / Hide menu item must always be available if shortcut registration fails.

### Window behavior

- Initial compact size: approximately **360 × 188 logical pixels**; adjust after measuring actual content.
- Small minimum size, no accidental cropping, and proper DPI scaling.
- Frameless/undecorated design **with accessible drag regions** and explicit close/hide control.
- Rounded corners, gentle shadow if reliably supported on Windows. Do not sacrifice launch reliability for transparency or blur effects.
- Always-on-top by default, switchable via tray/settings.
- Draggable by the header or other non-interactive drag regions; control buttons must remain clickable.
- Open near the **bottom-right** of the primary/current monitor on first show, with a sensible screen-edge margin. Reuse saved position after the user moves it.
- Restore or clamp the position onto a visible monitor after display/DPI changes or disconnected monitors.
- Retain a usable escape route even if the player is outside the visible screen.
- Prevent duplicate windows, duplicate tray icons, and accidental multiple resident processes when possible.

---

## 4. Visual direction — the most important part

The interface should feel like a tiny premium music widget, not a utility panel or default web app.

**Mood:** dark, cinematic, calm, clean, tactile, minimal. Spotify-adjacent, but an original UI without copying Spotify's branding, icons, or layout exactly.

### Design tokens

| Token | Suggested value |
| --- | --- |
| Panel background | `#111315` with subtle tonal variation |
| Elevated panel | `#1B1E20` |
| Text primary | `#F5F6F5` |
| Text secondary | `#9BA2A0` |
| Accent | `#1ED760` (editable, used sparingly) |
| Divider | translucent white at ~8% opacity |
| Corners | 18–22 px outer; 10–13 px inner |
| Font | system sans / Inter if bundled legally and efficiently |
| Spacing | 8 px grid, generous control hit targets |
| Motion | 120–220 ms, subtle easing; honor reduced-motion setting |

Use blurred color from the current album artwork **only as a restrained, low-opacity accent**, not a full-screen busy gradient. If the thumbnail is missing, maintain the same polished layout with a muted placeholder.

### Compact player layout (primary UI)

```text
┌──────────────────────────────────────────────────┐
│  ◉  NOW PLAYING                          ─    ×  │
│                                                  │
│  ┌──────────┐   Track title                      │
│  │          │   Artist name                      │
│  │  ALBUM   │                                    │
│  │  COVER   │   ◁◁       ▶ / ❚❚       ▷▷         │
│  └──────────┘                                    │
│                                                  │
│  1:26  ━━━━━━━━━●━━━━━━━━━━━━━━━━━━━━━━  3:42  │
└──────────────────────────────────────────────────┘
```

Implementation detail:

- Square album art on the left, ~88–96 px, with mildly rounded corners.
- Track details on the right, clean typography and truncation/tooltip for long strings.
- Main play/pause button visually dominant; previous/next understated.
- Small header with a subtle playing indicator and a compact hide button.
- Slim progress bar across the bottom with elapsed and total time **when available**.
- Hover states and keyboard-focus states for every interactive control.
- No scrollbar or wasted whitespace.

### Optional expanded view (after MVP)

A toggle can switch to a richer **album-art-focused** layout (~360 × 420 logical px) with a large centered square cover, clearer time scrubber, and controls along the bottom. Animation between compact/expanded must not cause flicker or offscreen positioning.

### Animation rules

- On show/hide: restrained fade + short translate/scale, where Windows/Tauri permits. Do not delay user input or block the window from showing.
- Track change: crossfade artwork and title, not a distracting long slide.
- Playing state: subtle pulsing/animated indicator, off when paused.
- Interactive buttons: soft hover fill and responsive press feedback.
- Respect `prefers-reduced-motion`.

Do **not** use copyrighted album art hardcoded into the repository. Show real artwork from the active Windows media session; use a neutral original placeholder otherwise.

---

## 5. Functional requirements

### 5.1 Session detection

States:

1. `spotify_not_running`: no Spotify session found; show **Open Spotify to begin**.
2. `spotify_detected_idle`: Spotify present but metadata not yet available; show a neutral waiting state.
3. `playing`: show accurate track data and pause icon.
4. `paused`: show accurate track data and play icon.
5. `disconnected`: Spotify was available, then disappeared; clear or mark stale data and offer retry.
6. `error`: native API failed; show concise diagnostic and retry action.

Spotify desktop application is the initial target. Other media sessions must not be accidentally controlled.

### 5.2 Player controls

- Play/pause: act on Spotify session; state updates from authoritative media-session playback information.
- Previous / next: call native session methods; disable if the media session doesn't support them.
- Repeated rapid clicks: no race conditions, no phantom UI updates.
- Track title and artist: accurate values from active session.
- Artwork: display if provided; placeholder otherwise.
- Timeline: show real elapsed/duration values where exposed. Interpolate cautiously between updates only while playing and only if known values support it.
- Seeking: enable *only if* a real timeline and seek support are available; otherwise render a non-interactive progress indicator. Never implement a fake draggable seek bar.
- Volume: **out of MVP scope** unless a verified and consistent Spotify-specific volume-control path is available; don't change the global system volume without explicit UX.

### 5.3 Settings

MVP minimum:

- Always on top toggle.
- Shortcut display and conflict status.
- Position reset to visible screen.

Post-MVP:

- Rebindable global hotkey.
- Compact/expanded default.
- Launch at Windows login (opt-in only).
- Theme/accent preference.
- UI opacity (subject to Windows window rendering).

### 5.4 Performance and safety

- Avoid needless high-frequency polling; use events where possible.
- A hidden window must not require high-frequency React rerenders.
- No unbounded album-art memory growth; release old artwork resources.
- No external telemetry, analytics, or credential collection.
- No Spotify username/password/API keys requested.
- Commands only act on the selected Spotify media session.
- No browser automation, DRM circumvention, stream capture, or audio redistribution.
- Avoid using `unwrap()` on fallible OS/session operations in production paths.

---

## 6. Suggested architecture

```text
SpotifyPiP/
├── README.md
├── package.json
├── index.html
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── PlayerWindow.tsx
│   │   ├── AlbumArt.tsx
│   │   ├── TransportControls.tsx
│   │   ├── ProgressTrack.tsx
│   │   └── EmptyState.tsx
│   ├── hooks/
│   │   └── useMediaSession.ts
│   ├── types/
│   │   └── media.ts
│   └── styles/
│       ├── tokens.css
│       └── player.css
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── lib.rs
│       ├── media_session.rs
│       ├── window_controller.rs
│       └── tray.rs
└── docs/
    └── WINDOWS_MEDIA_NOTES.md
```

Codex may adjust names to match official Tauri scaffolding conventions. Preserve clean separation between WinRT logic, Tauri IPC, and React rendering.

### Suggested native IPC contract

```ts
export interface MediaSnapshot {
  source: 'spotify' | 'none';
  status: 'playing' | 'paused' | 'idle' | 'disconnected' | 'error';
  title: string | null;
  artist: string | null;
  album: string | null;
  artworkDataUrl: string | null;
  positionMs: number | null;
  durationMs: number | null;
  canPlayPause: boolean;
  canNext: boolean;
  canPrevious: boolean;
  canSeek: boolean;
  errorMessage: string | null;
}
```

Suggested Tauri commands and events:

- `get_media_snapshot() -> MediaSnapshot`
- `media_play_pause() -> Result`
- `media_next() -> Result`
- `media_previous() -> Result`
- `media_seek(position_ms) -> Result` (only when verified)
- `set_always_on_top(enabled) -> Result`
- `reset_window_position() -> Result`
- Event `media://snapshot-updated` with the new snapshot.

Keep the backend the source of truth, and centralize error handling. Do not expose arbitrary shell execution or unrestricted file access through IPC.

---

## 7. Implementation plan for Codex

### Phase 0 — Bootstrap + verify

1. Create the **new** `SpotifyPiP` project via the official Tauri 2 React + TypeScript template, using npm.
2. Keep the project independent from the existing `JAM` directory.
3. Verify `npm install`, `npm run tauri dev`, and `npm run build` or the template's equivalent check.
4. Use a neutral original icon for the app; no misleading official Spotify branding.

### Phase 1 — Visual MVP with deterministic mock data

1. Build the compact floating player layout and robust empty/loading/error states.
2. Start with deterministic local mock values **only in a clearly separated development mode**.
3. Make the borderless/always-on-top floating window draggable and visually polished.
4. Implement show/hide, tray menu, and native global shortcut.
5. Check high-DPI scaling, minimum sizing, keyboard focus, and reduced-motion settings.

**Acceptance:** I can launch the app, press `Ctrl+Alt+M`, and show/hide a polished floating widget while VS Code is focused.

### Phase 2 — Real Spotify session integration

1. Build a small Rust media-session service and verify Spotify session discovery.
2. Log non-sensitive diagnostics during development: discovered source identifiers and API errors, not user secrets.
3. Implement actual metadata, playback state, thumbnail, and transport controls.
4. Push events to the React view; use a measured fallback polling interval only if needed.
5. Remove development mock track data from normal application mode.
6. Handle Spotify start/stop, track switches, pause, media-session restarts, and unsupported operations gracefully.

**Acceptance:** While the real Spotify desktop app is playing, the floating controller displays the real song and changes it when I press next. Opening YouTube in a browser does not make this app control YouTube instead.

### Phase 3 — Stabilize + package

1. Add window position persistence and an always-on-top preference.
2. Prevent accidental duplicate instances and fix tray lifecycle issues.
3. Ensure startup is hidden or minimized to tray by default, with an obvious first-run discovery method.
4. Package a Windows installer using `npm run tauri build`.
5. Document dependencies (WebView2, Windows compatibility), usage, shortcut, known Spotify/GSMTC limitations, and how to quit.
6. Run TypeScript checks, build checks, and a manual Windows smoke test.

**Acceptance:** A fresh launch of the packaged app restores the configured behavior, and the player functions with Spotify running without needing the development terminal.

---

## 8. Manual test matrix

| Scenario | Expected behavior |
| --- | --- |
| Spotify closed | Clear empty state; no playback commands sent to another app |
| Spotify playing | Correct track + playback icon + artwork when available |
| Spotify paused | Accurate paused state; play resumes when supported |
| Press shortcut in VS Code | Player appears above VS Code |
| Press shortcut again | Player hides; Spotify audio continues |
| Click X | Window hides; tray and shortcut remain active |
| Tray > Show / Hide | Works even if global hotkey is unavailable |
| Tray > Quit | Process, tray icon, and shortcut clean up |
| Song changes externally | UI updates accurately without manual refresh |
| YouTube/browser also playing | Spotify remains the selected controlled session |
| Missing artwork | Neutral original placeholder; no broken image |
| Timeline not available | No fabricated duration; disabled or hidden seek UI |
| Hotkey already taken | Clear fallback and recoverable settings |
| External monitor disconnected | Window can be brought back onto a visible screen |
| High DPI / text scaling | No clipping or tiny hit targets |
| Packaged installer | Behavior matches development mode |

---

## 9. Explicit non-goals for v1

Do **not** add the following until the core app is stable:

- Spotify login, OAuth, playlists, library browsing, or search.
- Embedding or resizing the actual Spotify client window.
- YouTube Music, Apple Music, or multi-player support.
- Custom audio decoding, downloads, streaming, or recording.
- Cloud backend, user accounts, analytics, or remote synchronization.
- JAM integration.
- Complex settings screens or an elaborate onboarding flow.

---

## 10. Instructions to Codex

> You are implementing this specification in a **new standalone repository**, not in JAM. Read the entire file before modifying anything. Use Tauri 2 + React + TypeScript + Rust. Build in the order Phase 0 → Phase 1 → Phase 2 → Phase 3. Prioritize the floating visual experience and the working global shortcut. Do not fabricate Spotify integration: use the native Windows GSMTC media-session API and verify what Spotify exposes on this machine. If a Windows API is unsupported or behaves differently in the packaged app, explain it and implement a graceful fallback rather than a fake feature. Keep code modular, avoid unnecessary dependencies, and run the relevant build/type checks after each phase. Start with Phase 0 and Phase 1; report what works, what needs live Spotify testing, and the commands to launch the app.

### First commands (run in a **new** parent directory, not inside JAM)

```powershell
npm create tauri-app@latest
# Project name: SpotifyPiP
# Package name: spotify-pip
# Identifier: com.devas.spotifypip
# Frontend: TypeScript / JavaScript
# Package manager: npm
# UI template: React
# UI flavor: TypeScript

cd SpotifyPiP
npm install
npm run tauri dev
```

After verifying the scaffold, add the Tauri plugin(s) needed by the chosen implementation; for the global hotkey:

```powershell
npm run tauri add global-shortcut
```

Follow official plugin setup, Rust registration, and Tauri capability permissions. For window positioning, consider the official positioner plugin only if it actually simplifies the implementation; Tauri window APIs may suffice.

---

## 11. Official technical references

- Tauri 2 Global Shortcut plugin: https://v2.tauri.app/plugin/global-shortcut/
- Tauri 2 Positioner plugin: https://v2.tauri.app/plugin/positioner/
- Tauri 2 Window State plugin: https://v2.tauri.app/plugin/window-state/
- Microsoft GSMTC session manager: https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager
- Microsoft GSMTC session controls and events: https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssession

**Reminder:** This is an unofficial personal Windows media-controller utility, not a Spotify product. Visual assets should be original, except for artwork supplied dynamically by Spotify's exposed Windows media session.
