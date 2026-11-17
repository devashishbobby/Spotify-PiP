# SpotifyPiP — Phase 2: Real Spotify Media Integration

**Codex implementation specification**  
**Version:** 1.0 — 17 November 2026  
**Project:** The **existing** standalone `SpotifyPiP` Tauri 2 + React + TypeScript application  
**Status:** Phase 0/1 (scaffold, player UI, floating window, tray, `Ctrl+Alt+M` hotkey) is assumed to be implemented. **Inspect the repository and verify this; do not assume exact file names.**  
**Primary reference:** Read `SPOTIFY_PIP_CODEX_SPEC.md` first. This document expands **only Phase 2** of that specification.

---

## 1. Goal

Replace the Phase 1 mock player data with **real, live playback information and working controls for the installed Spotify Windows desktop app**, through **Windows Global System Media Transport Controls (GSMTC)**.

The result should feel exactly like the polished floating widget from Phase 1, except the displayed track and transport buttons are now backed by Windows' actual Spotify media session.

**Core user story:**

> I start playing a song in the Spotify desktop app. I press `Ctrl+Alt+M` while coding, and SpotifyPiP appears with the **real song, artist, album art, playback status, and progress**. The play/pause and skip buttons control Spotify. Hiding the widget does not affect music playback.

### Mandatory acceptance criteria

1. With Spotify playing, the player displays the **actual title, artist, playback status**, and **album artwork when provided by Windows**.
2. Play/pause, previous, and next invoke real OS media-session commands targeting **Spotify only**, and reflect the subsequent authoritative playback state.
3. If Chrome, Edge, YouTube, or other media sessions are active, the app **does not accidentally control them**.
4. Song changes and play/pause actions made from Spotify itself are reflected in the widget without restarting the app.
5. Closing or restarting Spotify updates the app's state correctly. Opening Spotify after SpotifyPiP has started recovers automatically.
6. Missing thumbnails, missing timelines, disabled controls, unsupported WinRT APIs, and API errors render **honest, polished fallback states**.
7. `Ctrl+Alt+M`, the always-on-top window, drag behavior, tray menu, and hide-on-close lifecycle from Phase 1 still work.
8. Production UI displays **no mock song metadata**. Mock mode remains available solely for visual development and tests.
9. Both the frontend type/build checks and Windows Rust/Tauri build complete; explicitly report any live-device behaviors that could not be verified.

---

## 2. Scope and non-goals

### Implement now

- Rust/WinRT GSMTC media-session service for Windows 10/11.
- Spotify session enumeration and safe selection.
- Track metadata and artwork retrieval.
- Actual playback state and control availability.
- Play/pause, previous, and next commands.
- Timeline/progress display **when supported**, with limited interpolation.
- Optional seek **only if actually supported** and tested.
- Session and media-change listeners, with low-cost polling fallback.
- Frontend state management, typed Tauri IPC, error handling, and live UI states.
- Debug diagnostics, automated tests where practical, and manual Windows smoke-test guide.

### Do not implement

- Spotify OAuth, Spotify login, Spotify Web API, playlist browsing, or search.
- Audio playback/streaming/decoding inside SpotifyPiP.
- Capturing Spotify's audio, downloading tracks, or bypassing DRM.
- Controlling the master Windows volume or implementing an unverified Spotify-only volume slider.
- Supporting arbitrary browsers or other music services as controllable targets.
- Rebuilding JAM, changing JAM, creating a new Tauri scaffold, or rewriting the Phase 1 visual design.
- Phase 3 installers, autostart, multi-instance policy, and extensive preferences, except minimal fixes required by this integration.

**Priority:** Reliable playback state and safe targeting > artwork > progress > optional seeking. Do not sacrifice basic reliability for seek support.

---

## 3. Technical approach: Windows GSMTC

Use Microsoft's **`Windows.Media.Control`** WinRT API through the Rust `windows` crate (preferred), keeping all Windows-specific code behind a service boundary.

### Required API surfaces

| Requirement | Windows API |
| --- | --- |
| Get media-session manager | `GlobalSystemMediaTransportControlsSessionManager::RequestAsync()` |
| Enumerate ALL available sessions | `GetSessions()` |
| Identify source of a session | `GlobalSystemMediaTransportControlsSession::SourceAppUserModelId()` |
| Read title, artist, album, thumbnail | `TryGetMediaPropertiesAsync()` |
| Read playing/paused state and available buttons | `GetPlaybackInfo()` plus its control properties |
| Read timestamp, progress and duration | `GetTimelineProperties()` |
| Play/pause toggle | `TryTogglePlayPauseAsync()` |
| Next/previous | `TrySkipNextAsync()` / `TrySkipPreviousAsync()` |
| Seek (conditional) | `TryChangePlaybackPositionAsync(position_ticks)` |
| Detect Spotify opening/closing | Manager `SessionsChanged` event |
| Detect track change | Session `MediaPropertiesChanged` event |
| Detect playback change | Session `PlaybackInfoChanged` event |
| Detect timeline change | Session `TimelinePropertiesChanged` event |

**Critical:** Do **not** use `GetCurrentSession()` for playback commands; the current system-preferred session may be a browser. Enumerate and select the **Spotify** session explicitly.

### Dependency and capability verification

- Add the `windows` crate with **only the feature flags actually needed** after inspecting the APIs and installed crate version (likely including `Media_Control`, `Foundation`, `Foundation_Collections`, `Storage_Streams`, and any transitive media-related features required by compilation). Do not blindly paste feature flags from an outdated example.
- Keep the dependency Windows-gated using `cfg(target_os = "windows")` if useful.
- Handle WinRT initialization/thread affinity correctly. Run blocking `.get()` calls or other potentially slow WinRT work **off the Tauri UI thread**. Prefer a dedicated service/task/worker with predictable ownership, cancellation, and cleanup rather than starting a new unbounded thread per event.
- Microsoft documents `globalMediaControl` as an app capability for the WinRT surface. **Do not assume** that a packaged UWP capability manifest maps directly to an unpackaged Tauri Win32 process. Verify the actual access behavior on the user's Windows machine, record any errors, and avoid unsafe/undocumented workarounds.
- If a particular access route fails, capture a clear diagnostic and explain the compatibility limitation. Do not silently substitute an unrelated media source or browser automation.

---

## 4. Spotify session identification (safety requirement)

1. Request a session manager and enumerate `GetSessions()`.
2. For **development diagnostics only**, log the source application user model IDs (AUMIDs) of discovered media sessions, with timestamp and a concise status. Avoid private account data and avoid full track-title logging by default.
3. Determine the **real Spotify identifier(s) observed on this machine**. Spotify's Microsoft Store and conventional desktop builds may expose different IDs; **do not hardcode an unverified single identifier**.
4. Implement a conservative, documented matcher for verified Spotify application IDs. Use exact IDs or tightly scoped known Spotify variants where practical; do not select arbitrary processes just because a track mentions “Spotify.”
5. If a Spotify session exists, select it and retain a reference/identity for subsequent actions. On manager session changes, re-evaluate and rebind.
6. If multiple credible Spotify sessions are returned, apply a deterministic preference (e.g. the one actively playing, then valid metadata), or mark the state ambiguous and offer a safe resolution. Never accidentally target another app.
7. If none is found, return `spotify_not_running`. Do not fall back to controlling the current system session.
8. Before dispatching any playback command, confirm that the selected session is still valid and still corresponds to Spotify. Handle session disappearance without panicking.

**Definition of correct targeting:** Starting a YouTube video after Spotify starts must not switch SpotifyPiP to YouTube or make its play/pause/next buttons affect the browser.

---

## 5. Native service design

Inspect Phase 1 source first and integrate into the existing structure. Suggested logical modules (file names may differ):

```text
src-tauri/src/
├── media_session/
│   ├── mod.rs          # service lifecycle and public interface
│   ├── windows.rs      # WinRT/GSMTC access and event bindings
│   ├── state.rs        # snapshots, session identity, transitions
│   ├── artwork.rs      # stream to safe image payload
│   └── diagnostics.rs  # concise opt-in development diagnostics
└── lib.rs              # register commands, service, and lifecycle hooks

src/
├── types/media.ts
├── hooks/useMediaSession.ts
├── lib/mediaApi.ts
└── components/...      # reuse existing Phase 1 UI
```

### Lifecycle rules

- Initialize the service **once** at application startup, not on every React render and not on every show/hide.
- The service must remain able to track media while the window is hidden, without high-frequency UI updates.
- Register `SessionsChanged` and current Spotify session callbacks where supported; keep event registration tokens so handlers can be unregistered on session replacement or exit.
- Do not leak subscriptions or spawn duplicate listeners during Vite/Tauri hot reload, window toggles, or Spotify session replacement.
- Make callback handling nonblocking; schedule refresh work through a serialized/debounced worker to avoid reentrant WinRT operations.
- Keep the last known snapshot in managed Rust state, **but clear stale Spotify metadata** when the session genuinely disconnects or changes.
- Use distinct error states for `not_running`, `detected_idle`, `disconnected`, and `api_error`.
- Event handlers must not directly mutate React state. Rust pushes typed Tauri events; React subscribes and unsubscribes properly.
- Use a measured fallback refresh when events are missing/unreliable; e.g. around 1–2 seconds while the player is visible and Spotify is active, with a slower discovery cadence when absent/hidden. These are starting points, not strict numbers. Prefer event-driven refresh over permanent fast polling.
- Commands should be serialized or protected against stale session races. A successful WinRT request indicates it was accepted, **not necessarily that the UI can optimistically claim final playback state**.

---

## 6. Strongly typed frontend/backend contract

Align with the existing project's type names if they already exist. Define shared semantic fields in both Rust Serde and TypeScript and keep them consistent.

Suggested extended contract:

```ts
export type SessionState =
  | 'spotify_not_running'
  | 'spotify_detected_idle'
  | 'playing'
  | 'paused'
  | 'disconnected'
  | 'error';

export interface MediaSnapshot {
  source: 'spotify' | 'none';
  state: SessionState;
  title: string | null;
  artist: string | null;
  album: string | null;
  artworkDataUrl: string | null;

  positionMs: number | null;
  durationMs: number | null;
  lastTimelineUpdateUtcMs: number | null;

  canPlayPause: boolean;
  canNext: boolean;
  canPrevious: boolean;
  canSeek: boolean;

  errorCode: string | null;
  errorMessage: string | null;
  revision: number; // increment on meaningful native state changes
}
```

**Compatibility:** If Phase 1 already defined `status` rather than `state`, do not break it gratuitously. Either preserve the existing field or implement an explicit, single adapter. No scattered optional-field guesses throughout components.

### IPC endpoints

Expose typed Tauri commands, adapting command names if Phase 1 already uses them:

```text
get_media_snapshot()                     -> Result<MediaSnapshot, AppError>
media_play_pause()                       -> Result<CommandResult, AppError>
media_next()                             -> Result<CommandResult, AppError>
media_previous()                         -> Result<CommandResult, AppError>
media_seek(position_ms: integer)         -> Result<CommandResult, AppError>  [ONLY IF VERIFIED]
media_refresh()                          -> Result<MediaSnapshot, AppError>   [optional manual retry]
```

Suggested `CommandResult`: `accepted: boolean` and an optional normalized error/message. After accepted actions, refresh **authoritative** OS playback state instead of pretending a command succeeded based on the click alone.

Tauri event:

```text
media://snapshot-updated
```

Event payload: `MediaSnapshot`. Register the frontend event listener with Tauri's `listen` API and clean up the unlisten callback on component unmount. Subscribe before obtaining the initial snapshot (or reconcile by `revision`) to avoid lost updates. Ignore out-of-order snapshots by revision.

**Error handling:** Use serde-serializable structured errors where feasible; never expose raw panic traces or sensitive internals in the UI. Do not add unrestricted shell commands or filesystem permissions.

---

## 7. Metadata and artwork

### Track information

Read `TryGetMediaPropertiesAsync()` and map actual available properties:

- `Title` → track name.
- `Artist` → artist name.
- `AlbumTitle` → album, if present.
- `Thumbnail` → album art, if present.

Handle empty or unavailable strings as `null` or an intentional placeholder. Long titles must preserve the Phase 1 UI layout (truncate visually; full accessible text/tooltip if needed).

### Thumbnail handling

- Windows may expose a `RandomAccessStreamReference` thumbnail. Open/read the stream safely and convert it to an image payload the WebView can display.
- Use correct binary/WinRT stream-reading APIs; do **not** assume the thumbnail is a URL.
- Identify/validate MIME type where practical; accept only supported image types.
- Put a sensible cap on bytes/pixel dimensions; avoid unbounded buffer allocation or unlimited base64 sizes.
- Avoid unnecessary artwork rereads while the track remains the same. Cache only the current image or use a small bounded cache, and release replaced images.
- Do not persist artwork or bundle third-party album covers into the repository.
- On absent/failed artwork, show the polished generic placeholder from Phase 1; metadata and transport controls should still work.
- If base64 is too heavy, use a carefully scoped safer internal artwork-serving mechanism, but do not introduce a local HTTP server solely for album art without a clear justification.

**Do not fetch album artwork from arbitrary external search results, unofficial scraping sites, or the Spotify Web API.**

---

## 8. Playback controls

### Play/pause, next and previous

- Query control availability from the current `GetPlaybackInfo()` control flags and disable unsupported buttons.
- Use `TryTogglePlayPauseAsync`, `TrySkipNextAsync`, and `TrySkipPreviousAsync` on the **selected Spotify session**.
- Treat `false` / failure / OS error as a real failure. Show subtle feedback (tooltip or short inline status) and leave UI consistent with OS state.
- Do not let clicks overlap unpredictably. While an action is pending, either briefly disable the affected control or serialize commands.
- Avoid introducing perceptible lag by blocking the React/Tauri thread.
- Controls must have keyboard labels, hover/focus states, and the same tactile interaction styling as Phase 1.

### Timeline and seeking

- Read timeline `StartTime`, `EndTime`, `Position`, `LastUpdatedTime` and valid seek bounds when returned.
- **WinRT time values are typically 100-nanosecond ticks**; convert carefully to milliseconds using checked/saturating arithmetic. Do not treat ticks as milliseconds.
- Derive the displayed duration from valid `EndTime - StartTime`; handle invalid, zero, unknown, or nonfinite duration as unavailable.
- Convert progress to a fraction of the valid timeline range, not blindly `Position / EndTime` if the start time is nonzero.
- If the session is playing, interpolate a display-only progress position based on a received anchor and elapsed monotonic time, clamped to a known duration. Do not run a native OS query every animation frame.
- When paused, stop interpolation. On track change, reset the anchor. Periodically reconcile against the OS snapshot, including when the window is shown after being hidden.
- Enable scrubbing **only** after verifying that the OS session supports seeking and that `TryChangePlaybackPositionAsync` works on the installed Spotify build. Seek API input is in ticks, not milliseconds. Clamp to allowed range.
- If seeking is unavailable, show a noninteractive visual progress bar and do not style it as draggable. Show `--:--` or omit time labels when unknown; **never invent track times**.

---

## 9. React integration and UI states

**Preserve the existing premium dark floating design.** Replace only the data layer and state-dependent affordances.

### `useMediaSession()` behavior

1. Attach the `media://snapshot-updated` listener once.
2. Invoke `get_media_snapshot()` for initial state; reconcile using the `revision` field.
3. Expose the current snapshot, a derived view-model, `playPause()`, `next()`, `previous()`, `refresh()`, and pending-action/error flags.
4. Treat Rust's snapshot as source of truth. Do not leave Phase 1 mock data as an implicit fallback when the real session is absent.
5. Clean up event subscriptions and local display intervals on unmount. Do not recreate listeners on every track change.
6. Use efficient progress updates only while the window is visible or when the UI genuinely needs them.
7. Sanitize and bound metadata strings as necessary; React's default text rendering should be used rather than injecting HTML.

### States to design/verify

| State | Visible result | Controls |
| --- | --- | --- |
| Spotify not running | “Open Spotify to begin” + subtle empty artwork | Disabled |
| Spotify detected, no track | “Waiting for playback” | Disabled or availability-based |
| Playing | Real title, artist, art, pause icon, progress if available | Based on OS flags |
| Paused | Real title, artist, art, play icon, frozen progress | Based on OS flags |
| Session disconnected | Clear stale metadata and show “Spotify disconnected” | Disabled |
| Artwork unavailable | Generic artwork placeholder; keep real text | Other controls still work |
| Timeline unavailable | No fabricated elapsed/total; static or absent progress | Seek disabled |
| Native API failure | Short readable message and Retry action | Disabled until recovered |

Do not display “Connected to Spotify” if the app only sees a generic media session or has not confirmed an actual Spotify source.

---

## 10. Diagnostics and Windows validation

Create `docs/PHASE_2_WINDOWS_TESTS.md` containing concise test instructions and a final results table.

### Diagnostic details to capture (development only)

- Windows OS version/build.
- App package/executable environment (development vs packaged, if known).
- Whether `RequestAsync()` succeeded.
- Number of sessions enumerated.
- Source AUMIDs sufficient to diagnose Spotify targeting (no usernames, tokens, passwords, or personally identifying content).
- Chosen Spotify session identity.
- Whether metadata/artwork/timeline/control flags were available.
- Meaningful WinRT HRESULT/error category if an operation fails.

Never continuously spam the console with per-frame logs. Add a small opt-in debug flag if needed; redact private data.

### Required manual smoke tests

| ID | Test | Expected |
| --- | --- | --- |
| T01 | Launch SpotifyPiP while Spotify is closed | Clear not-running state, no fake track |
| T02 | Open Spotify and play a song | Automatically detect and show real metadata |
| T03 | Pause in Spotify | Widget changes to paused without manual reload |
| T04 | Play/pause from widget | Spotify actually responds, UI follows real state |
| T05 | Next/previous from widget | Spotify changes track, widget follows |
| T06 | Play YouTube in Chrome/Edge at same time | Spotify remains the only controlled target |
| T07 | Switch song directly in Spotify | Widget updates title, artwork and timeline |
| T08 | Close Spotify while widget is visible | Disconnected/not-running state; controls disabled |
| T09 | Reopen Spotify after closing it | Automatic rediscovery and recovery |
| T10 | Artwork absent or thumbnail read fails | Clean placeholder without crash |
| T11 | Duration/progress/seek unsupported | No fake numbers; noninteractive progress |
| T12 | Toggle `Ctrl+Alt+M` in VS Code | Window shows/hides; audio uninterrupted |
| T13 | X/hide, restore from tray, Quit | Phase 1 window and tray behavior preserved |
| T14 | Rapid skip/play interactions | No crash, wrong-player action, or stuck spinner |
| T15 | Leave running through multiple tracks | No duplicate events, runaway memory use, or CPU spikes |
| T16 | Reload dev frontend / restart Spotify | No leaked or duplicate listeners |

If the live Spotify app is not available to Codex or hardware tests cannot be performed, say **“not verified on this machine”** instead of marking them passed.

### Automated tests to add where feasible

- Session-ID matcher accepts verified Spotify IDs and rejects browser/other app IDs.
- Snapshot mapping handles no song, incomplete metadata, unknown duration and unavailable controls.
- Timeline conversion and clamping handle 100ns ticks, nonzero start times, pauses, and invalid durations.
- Commands reject a stale/disconnected/non-Spotify session.
- React hook/view-model tests for transition from mock-free `not_running` → `playing` → `paused` → `disconnected`.

Keep OS-specific integration behind a testable adapter/interface so deterministic unit tests can use fakes without a live Spotify install.

---

## 11. Order of implementation — do not skip ahead

### Task A — Inspect and protect Phase 1

- Read `SPOTIFY_PIP_CODEX_SPEC.md` and inspect existing source and commands.
- Identify current mock data, player hook, Tauri initialization, tray, shortcut, and window behavior.
- Establish baseline `npm install` (only if needed), `npm run build`, and `cargo check` or `npm run tauri dev`.
- Do **not** delete/recreate the project. Do not overwrite existing UI styling or app identifier.

### Task B — Prove access to Windows media sessions

- Add the Windows dependency, create a minimal native service, and verify `RequestAsync()` and `GetSessions()` against actual Windows.
- Capture observed Spotify source IDs; implement conservative selection; test against browsers.
- If API access is blocked, troubleshoot using official docs and report exact failure before investing in higher layers.

### Task C — Live media snapshot

- Read selected session's media properties and playback info.
- Normalize fields into the typed Rust/TypeScript snapshot.
- Replace production mock data with the live initial snapshot.
- Implement clear no-session and error states.

### Task D — Real transport actions

- Implement play/pause, next and previous commands with session-identity validation and availability checks.
- Update UI from OS snapshots, not optimistic fake actions.
- Verify all three on installed Spotify desktop client.

### Task E — Events and recovery

- Add manager and session event handlers, teardown/rebind logic, fallback refresh and deduplication.
- Verify app startup before Spotify, Spotify restarts, track changes, and window hide/show.

### Task F — Artwork and timeline

- Implement safe thumbnail reading and bounded artwork handling.
- Add genuine timeline and interpolation.
- Consider seek **only after everything else is stable and support is confirmed**.

### Task G — Validation and handoff

- Run frontend TypeScript/build check, Rust check, and Tauri dev build.
- Execute or document manual T01–T16 tests accurately.
- Write `docs/PHASE_2_WINDOWS_TESTS.md` and concise README notes for Spotify compatibility.
- Summarize changed files, completed features, tests passed, unverified tests, remaining issues, and exact start command.

---

## 12. Exact Codex prompt

Paste the following into Codex **from the existing `SpotifyPiP` repository root**:

> Read `SPOTIFY_PIP_CODEX_SPEC.md` and `SPOTIFY_PIP_PHASE_2_CODEX_SPEC.md` in full. Phase 0/1 already created the Tauri 2 + React + TypeScript floating-player app. Your task is to implement **Phase 2 only**: real Spotify desktop media-session detection, metadata, artwork, play/pause/next/previous, OS-derived playback state, events/recovery and genuine progress where available, using Rust's Windows GSMTC APIs. Preserve the existing UI, system tray, window behavior and `Ctrl+Alt+M` global shortcut. Do not create a new project, modify JAM, add Spotify OAuth/Web API, control other players, or leave mock track data in production. Work through Tasks A–G in order. Verify the installed WinRT API signatures, build the app, test against Spotify if accessible, and document unverified hardware behavior honestly. Do not stop at writing a plan; implement the code now and report changes and test results.

---

## 13. Official references

- Microsoft GSMTC session manager (session enumeration and change events):  
  https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessionmanager
- Microsoft GSMTC session (metadata, commands, events):  
  https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssession
- Microsoft timeline properties (position, start/end, last updated):  
  https://learn.microsoft.com/en-us/uwp/api/windows.media.control.globalsystemmediatransportcontrolssessiontimelineproperties
- Rust `windows` crate WinRT binding documentation:  
  https://microsoft.github.io/windows-docs-rs/doc/windows/Media/Control/struct.GlobalSystemMediaTransportControlsSession.html
- Tauri 2 frontend ↔ Rust commands/events:  
  https://v2.tauri.app/develop/calling-rust/

**Compatibility reminder:** This app is an independent unofficial Windows utility; Spotify controls and metadata depend on what the installed Spotify client exposes to Windows. Do not represent unsupported actions or fields as working.
