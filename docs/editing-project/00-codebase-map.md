# 00 — Codebase map (Playlist editing project)

Written 2026-11-10 from commit `b03e0cc` on branch `playlist-enhancements`. No source files were changed to produce it. Paths are relative to the repo root.

How to read this: sections 1–9 describe what exists today. Section 10 lists the places where the code contradicts or complicates the project brief. Section 11 lists what I did and did not verify.

---

## 1. Platform and layout

- **Desktop app: Tauri 2.11** (Rust crate `tauri 2.11.6`, JS `@tauri-apps/api ~2.11.1`). The Rust crate is in **`engine/`**, not `src-tauri/`. It is a single crate named `app` (`engine/Cargo.toml`), with no workspace. `rust-version = "1.86"`, and the local compiler is `rustc 1.86.0`.
- **Frontend:** React 18.0, TypeScript 4.6 (`strict: true`), Vite 2.9, Redux Toolkit 1.8, Tailwind 3 plus SCSS. The package manager is pnpm (lockfile v6, so pnpm 8).
- **Playlist rendering:** one p5.js canvas through `react-p5-wrapper`. Clips are not DOM elements.
- **Audio crates:** `rodio 0.19` (which brings `cpal 0.15.3`), `hound 3.5` (WAV writing), `symphonia 0.5` (MP3 inside rodio), `rtrb 0.3` (lock-free queues). There is **no time-stretch, pitch-shift or resampling crate**.
- **Licence:** `engine/Cargo.toml` says `license = "MIT"`. There is no LICENSE file in the repo.
- **Running it:** `pnpm ui:start` (Vite on port 4431) and `pnpm dev` (`tauri dev`) in separate terminals. `beforeDevCommand` is empty, so `pnpm dev` alone does not start Vite (`engine/tauri.conf.json`).

Where things live:

| Area | Location |
|---|---|
| Audio engine core | `engine/src/daw/daw_core/` (`engine.rs`, `audiograph.rs`, `clip.rs`, `mixer.rs`, `sequencer.rs`, `input.rs`, `sound_buffer.rs`, `timing.rs`, `waveform.rs`) |
| Transport and scheduler | `engine/src/daw/daw.rs` |
| Shared engine state | `engine/src/daw/state/` (`state.rs`, `playlist.rs`, `defaults.rs`) |
| Tauri commands | `engine/src/event_dispatch.rs`, `sequencer_commands.rs`, `mixer_commands.rs`, `audio_commands.rs`, `project.rs`, `recent.rs`; registered in `engine/src/main.rs:83-179` (95 commands) |
| Project files | `engine/src/project.rs`, `engine/src/project_files.rs` |
| Native menu | `engine/src/app/window.rs` |
| Redux store | `src/state/store.ts`, `src/state/slices/*.ts` |
| Playlist view | `src/views/playlist/` (`PlaylistCanvas.tsx`, `ClipMenu.tsx`, `sketch/`) |
| Skeuomorphic controls | `src/components/fourtrack/` (`AnalogButton`, `Led`, `Knob`, `MasterFader`, `MuteSolo`, `LevelMeter`, `TapeCounter`, `RollingCounter`, `Cassette`), styles in `fourtrack.scss` |
| Key helpers | `src/util/keys.ts` |

---

## 2. How clips are modeled

### 2.1 Engine side

A sample clip is an `AudioNode` (`engine/src/daw/daw_core/audiograph.rs:172-188`):

```rust
pub struct AudioNode {
  pub id: u64,
  pub start_offset: Duration,     // time from the playlist start
  pub track_number: u32,
  sample_path: String,
  channels: u16,
  buffer: daw::sound_buffer::SampleBuffer,   // the file's encoded bytes
  duration: Duration,             // the whole file's length
  sample_rate: u32,
  waveform: String,
  settings: daw::ClipSettings,
}
```

A pattern clip is a `PatternNode` (`audiograph.rs:350-358`): `id`, `pattern_id`, `start_offset`, `track_number`. It has no length of its own (it is as long as its pattern) and no settings.

Both kinds live in `AudioGraph` (`audiograph.rs:396-414`) as two lists, `nodes` and `pattern_nodes`, sharing one id counter (`next_id`). Ids are never reused within a session, and they are **not saved**: every project load gives every clip a new id (`project.rs:295-297`).

Per-clip settings are `ClipSettings` (`engine/src/daw/daw_core/clip.rs:9-22`, mirrored in `src/types/clip.ts`):

| Setting | Range | Notes |
|---|---|---|
| `gain_db` | −60 to +12 dB | |
| `pan` | −1 to 1 | balance law, output is always stereo |
| `pitch` | −24 to +24 semitones | **also changes speed and length**, like tape (`playback_rate = 2^(pitch/12)`, `clip.rs:50`) |
| `attack_ms` | 0 to 10 000 | linear fade in from the clip's start |
| `decay_ms` | 0 to 10 000 | linear fade out to the clip's end |

Fades are clamped to the clip's played length (`ClipSettings::clamped`, `clip.rs:66-81`). The clamp changes the stored value.

What does **not** exist per clip: source offset or trim start, trim end or length, independent speed or time-stretch, reverse, name, colour, mute.

### 2.2 How a clip refers to its audio

By file path only (`AudioNode.sample_path`). There is no sample pool and no source id. `AudioNode::new` (`audiograph.rs:227-265`) reads the whole file into its own `SampleBuffer` (`Arc<Vec<u8>>` of the **encoded** file, `sound_buffer.rs:10-27`), then decodes the file a second time just to measure its length and build the waveform. Two clips of the same file each hold their own copy of the bytes. Nothing in the engine knows that two clips "share a source" beyond having equal path strings.

### 2.3 Frontend side

`PlaylistItem` (`src/types/playlist.ts:17-28`): `id`, `path`, `startBeat`, `trackNumber`, `pixelOffset` (legacy, debug only), and `patternId?` (set on pattern clips, whose `path` is `''`).

Settings are held beside it, keyed by clip id, in `PlaylistState.clipSettings` and `PlaylistState.clipSampleDurations` (`src/state/slices/playlistSlice.ts:34-63`).

The engine is the source of truth. Every edit is a thunk that awaits `invoke(...)` and then dispatches what the engine returned, for example `moveNodeInPlaylist` (`playlistSlice.ts:468-499`).

### 2.4 Time units

- The engine stores positions as **time** (`Duration`), not beats or ticks. There is no PPQ.
- The frontend and the project file use **beats**. Conversion: `beat_to_offset` and `offset_to_beat` (`event_dispatch.rs:313-333`), and `capture`/`apply` in `project.rs`.
- One beat is always a quarter note: `60 / tempo` seconds (`timing.rs:66-71`). The time signature only affects beats per bar.
- Snap values: whole, half, quarter, eighth, sixteenth note (`timing.rs:18-37`). Default: snap on, quarter note. The engine applies snapping (`find_nearest_offset_subdivision`, `audiograph.rs:977-991`); the frontend mirrors the rule only to draw previews (`Renderer.snapBeat`, `sketch/index.ts:531-536`).
- **The playlist is 16 beats long.** `MAX_BEATS` and `MAX_BEATS_DISPLAYED` are both 16 (`state/defaults.rs`) and nothing changes them at run time. Every add and move clamps the clip's start into 0–16 beats (`calc_sample_offset`, `engine/src/app/workspaces/playlist.rs:8-27`; `group_move`, `audiograph.rs:1098-1129`). A clip's end may hang past beat 16.

### 2.5 Commands that change clips today

| Action | Thunk (`playlistSlice.ts`) | Tauri command |
|---|---|---|
| Add sample clip | `addToPlaylist` :412 | `t_add_audiograph_node` |
| Add pattern clip | `addPatternToPlaylist` :445 | `add_pattern_clip` |
| Move one clip | `moveNodeInPlaylist` :468 | `move_audiograph_node` |
| Move a selection | `moveNodesInPlaylist` :512 | `move_audiograph_nodes` |
| Delete one | `removeFromPlaylist` :547 | `remove_audiograph_node` |
| Delete a selection | `removeManyFromPlaylist` :533 | `remove_audiograph_node`, once per clip, in sequence |
| Change settings | `updateClipSettings` :660 | `set_clip_settings` (throttled to one send per 50 ms) |
| Explode pattern clip | `explodePatternClips` (`sequencerSlice.ts:225`) | `explode_pattern_clips` |

Recording also adds clips (`recording::finish`, `engine/src/recording.rs:243-305`).

Clips may overlap on a track. Nothing prevents it, and overlapping clips all play.

---

## 3. How the Playlist renders and handles input

### 3.1 Structure

- `PlaylistCanvas.tsx:32` creates one module-level `Renderer` (`src/views/playlist/sketch/index.ts:198`). The renderer owns **all interaction state**: zoom (`currentScale`), pan (`transformX`), vertical scroll (`scrollY`), the selection (`selectedIds: Set<number>`, :263), the lasso, and drags. None of this is in Redux.
- Drawing classes: `PlaylistTrack`, `PlaylistObject` (one clip: card, label band, modified dot, pattern cells), `Waveform` (waveform and the clip's geometry), `Timeline` (ruler), `Cursor` (playhead).
- Clip width is always `secondsToWidth(duration)` (`Waveform.ts:72`), where `duration` is the played length. The clip's hit box is the whole lane height. There are no edge zones.
- Coordinates: at zoom 1 the canvas width spans `beatsDisplayed` (16) beats. `beatToX`, `xToBeat`, `screenToPlaylistX`, `screenXToBeat` are at `sketch/index.ts:784-938`. Zoom is horizontal only, minimum 1, no maximum.
- Colours are RGB tuples in `sketch/theme.ts` (slate, sky and amber, matching the four-track controls).
- The canvas draws only when something changes (`wake`, `#animating`, `sketch/index.ts:469-486`), at up to 60 frames per second.

### 3.2 Mouse gestures today

All handlers are p5 mouse handlers in `sketch/index.ts` (`canvas.mousePressed` :991, `canvas.mouseMoved` :1118, `canvas.mouseReleased` :1090, window-level `p.mouseReleased` :1046). There is no touch handling.

| Gesture | Result | Where |
|---|---|---|
| Drag on empty space | **Pans** the view (horizontal and vertical) | :1127-1130 |
| Click on empty space | Clears the selection | :1105 |
| Drag a clip (past 10 px) | Moves it; a selected clip brings the whole selection | :1026-1038, :1124-1126 |
| Press an unselected clip | Clears the selection (the pressed clip is **not** selected) | :1024 |
| **Shift**+click a clip | Toggles that clip in the selection | :1012-1013 |
| **Shift**+drag on empty space | Lasso (selection box), always adds to the selection | :1014, `selection.ts` |
| Right-click a clip | Opens `ClipMenu` | :940-951, `PlaylistCanvas.tsx:43-80` |
| Double-click a clip | Sample clip: opens the Sample tab. Pattern clip: opens the Sequencer | :1165-1168 |
| Drag in the ruler | Moves the playhead (snapped) | :366-385 |
| Wheel / two-finger swipe | Scrolls; Shift+wheel scrolls sideways | :1061-1087 |
| Cmd or Ctrl + wheel, trackpad pinch | Zooms around the mouse | :1071-1076, :1150-1162 |
| Release outside the canvas | Cancels a clip drag | :1046-1057 |

Details that matter for this project:

- **The lasso requires Shift.** Without Shift, dragging on empty space pans.
- **Why a lasso cannot start over a clip:** in `canvas.mousePressed`, a Shift-press with a clip under the mouse takes the "toggle this clip" branch and returns (`sketch/index.ts:1012-1016`). The lasso is only started in the `else`.
- The lasso has no drag threshold: it exists from the moment of the press.
- The lasso's hit rule is "the box touches the clip's lane-height box" (`rectsTouch`, `selection.ts:36-40`).
- The drag threshold for clips is 10 px (`staticDefaults.mouseDragDetectionThreshold`).
- **No modifier key disables snapping.** Snap is toggled with the Snap key in the topbar.
- The move is sent to the engine once, on release. The engine snaps and returns the final position.
- Cursors are standard only: crosshair while lassoing, pointing hand over a clip or the ruler, `grabbing` while dragging (`sketch/index.ts:1418-1430`).
- There is **no tool or mode concept** anywhere in the app.

### 3.3 Selection

- Lives in `playlistRenderer.selectedIds`. Selected clips are drawn in sky colours with a 1.5 px border (`PlaylistObject.ts:152-158`).
- Operations on a selection today: group move, Delete or Backspace, Shift+E (explode the selected pattern clips).
- Escape clears the selection from anywhere in the window. That handler (`p.keyPressed`, `sketch/index.ts:1136-1141`) has no focus or typing guard.

### 3.4 Context menu

`src/views/playlist/ClipMenu.tsx` is a plain white Tailwind popover (it is not styled like the four-track controls). Items are defined in `PlaylistCanvas.tsx:58-77`:

- Sample clip: **Delete** only.
- Pattern clip: **Explode into tracks** (⇧E), **Open in Sequencer**, **Delete**.

Items that do not apply are left out, not greyed. The menu acts on the right-clicked clip only, even when other clips are selected. Right-clicking does not change the selection. Right-clicking empty space does nothing. The menu has no separator support.

### 3.5 Topbar controls that a tool switcher would sit beside

`src/components/topbar/ButtonArea.tsx` has two recessed trays (`ft-tray`): Record / Play-Pause / Stop, and Metronome / Count-in / Loop / Snap. The toggles are `LatchKey`s (`LatchKey.tsx`): an extra-small `AnalogButton` with a blue `Led` above it. A radio-style group of latched keys already exists in `src/views/sequencer/PatternStrip.tsx:156-168`.

---

## 4. How the audio engine plays clips

### 4.1 Output and threading

- One cpal output stream, opened lazily at the **device's sample rate** (`engine.rs:533-682`). The callback runs `Renderer::render` (`engine.rs:357-419`), which mixes voices into interleaved stereo `f32`.
- The callback takes no locks. Voices arrive through a lock-free queue as `Command::Start`, `Command::Stop` or `Command::Play` (`engine.rs:56-63`).
- **`Engine::new(sample_rate)` returns an `(Engine, Renderer)` pair with no device** (`engine.rs:121-149`). Tests drive `renderer.render(...)` by hand. This is the basis for offline rendering and numeric tests.

### 4.2 Scheduling

- `daw::run_playlist` (`engine/src/daw/daw.rs:167-364`) starts a scheduler thread per run. It works in one-beat slices and sends each slice's voices about 50 ms plus two device buffers early, each tagged with its exact start frame.
- For each slice, `AudioGraph::slice_sounds` (`audiograph.rs:502-545`) picks every clip whose `start_offset` falls in the slice. On the first slice of a run it also picks clips already sounding and skips into them with `skip_duration`.
- A clip's sound is `AudioNode::source()` (`audiograph.rs:298-300`): a fresh rodio `Decoder` over the file's bytes, converted to `f32`, then `ClipSettings::apply` (`clip.rs:86-98`): speed → gain → fade envelope → pan (always stereo). It then passes through the track's mixer strip (`Mixer::track_strip`), the master strip and a soft limiter.
- **A clip always plays its whole file.** There is no start offset and no length limit. A voice ends when its decoder runs out.
- A voice is cut early only by stop or pause (10 ms fade). Clips are not cut at the loop point.
- **Setting changes take effect the next time the clip starts** (`event_dispatch.rs:588-599`). A clip that is already sounding is not changed.
- Pattern clips are expanded into one voice per drum hit in each slice (`audiograph.rs:526-542`). A hit plays its sample to the end, even past the end of the pattern clip.

### 4.3 Sample data, pitch and quality

- Decoding happens lazily as the audio callback pulls samples, from memory (no disk access).
- **rodio 0.19's decoder outputs 16-bit integers** (`type Item = i16` in rodio's `decoder/mod.rs`). All audio, including the app's own 32-bit float recorded takes, is reduced to 16 bits when it is played.
- Pitch uses rodio's `Speed`, which only relabels the sample rate (truncated to whole Hz). The engine then converts every voice to the device rate with rodio's `UniformSourceIterator`, which uses **linear interpolation** with no filter (`engine.rs:193`).
- There is no time-stretch. `src/types/clip.ts:3-5` says so: "independent pitch and speed would need time-stretching, which can come later".

### 4.4 Transport

| Action | Function | Behaviour |
|---|---|---|
| Play | `start_playlist` (`daw.rs:426-452`) | Plays from the playhead. A playhead at or past the loop end (or song end, with looping off) starts from 0 |
| Pause | `pause_playlist` (`daw.rs:400-414`) | Stops and **keeps** the playhead |
| Stop | `stop_playlist` (`daw.rs:417-424`) | Stops and **returns the playhead to 0** |
| Seek | `seek_playlist` (`daw.rs:491-504`) | Clamped to 0–16 beats; carries on playing if playing |
| Loop | `Playlist::loop_len` (`state/playlist.rs:161-167`) | On by default; always 0–16 beats; no loop markers |

There are three play controls with different behaviour:

- Topbar Play/Pause calls `togglePlayHere` (`src/state/slices/sequencerSlice.ts:248-265`): if the Sequencer is the focused view and a pattern is selected, it loops that pattern; otherwise it toggles play/pause of the playlist.
- Topbar Stop calls `stopPlaylist`.
- The Transport tab has separate Play, Pause and Stop keys (`src/views/transport/TransportDeck.tsx`).

Ending a run also ends any recording in progress (`end_run`, `daw.rs:367-389`).

### 4.5 Offline render or bounce

**None exists.** The only code that writes audio is the recording path, which writes 32-bit float WAV at the input's rate with `hound` (`engine/src/daw/daw_core/input.rs:233-290`). `AudioNode::source()` already yields a clip's processed audio as an iterator, so a bounce can be built on it, but note that after `speed()` the source reports a relabelled sample rate, so a file writer must resample and cannot just copy samples.

---

## 5. Projects on disk

- **Format:** pretty-printed JSON, extension `.dawesome`, written atomically through a `.tmp` file (`project.rs:421-435`).
- **Top-level struct:** `Project` (`project.rs:77-95`): `version`, flattened `ProjectSettings` (tempo, time signature, metronome, loop, snap, count-in), `clips`, `mixer`, `sequencer`, `patternClips`, `armed`.
- **A saved clip** is `ProjectClip` (`project.rs:53-63`): `path`, `trackNumber`, `startBeat`, `settings`.
- **Layout:** a project is a folder named after it (`project_files.rs:1-15`):

  ```
  my_project/
    recordings/
      my_project_11092026_0.wav
    my_project.dawesome
  ```

  Older bare `.dawesome` files still load and save in place, with `recordings/` beside them.
- **Paths:** a sample inside the project folder is stored relative to it; any other sample is stored as an absolute path (`stored_path`, `project.rs:201-217`). Samples from the browser are **not** copied into the project.
- **Unsaved projects:** `ProjectLocation.path` is `None` (`project_files.rs:27-33`). Recordings made before the first save go to `<app data>/dawesome/unsaved/<YYYYMMDD>-<hex>/` and are moved into `recordings/` on the first save; Save As copies them (`relocate_takes`, used by `save_project_to`, `project.rs:520-580`).
- **There is no `clips/` folder** in the code. `docs/recording.md` mentions `exports/ (later)`.
- **Versions:** `PROJECT_VERSION = 2`. A project with no patterns is written as version 1 so that older builds can still open it (`project.rs:19-22`, `:275-277`). A build refuses any file with a higher version than it knows (`validate`, `project.rs:121-140`). There is no migration code; missing fields take serde defaults.
- **File naming helpers that already exist:** `next_take_paths` and `free_path` (`project_files.rs:172-196`) pick the next free name and never overwrite.
- **Deleting files:** recordings deleted in the browser go to the OS trash through the `trash` crate (`audio_commands.rs:375,390`).
- **All file writing is in Rust commands.** The frontend's only filesystem permission is `fs:allow-read-dir` (`engine/capabilities/default.json`). Open and save dialogs are shown from the frontend with `@tauri-apps/plugin-dialog` (`src/state/slices/projectSlice.ts`).
- **Messages to the user:** `src/components/ProjectNotice.tsx` is a dismissable banner with info, warning and error styles.

---

## 6. Undo/redo

**It does not exist**, on either side. There is no history, no command pattern and no snapshots. There is also **no "unsaved changes" flag**: `okToReplace` (`projectSlice.ts:99-108`) asks before replacing the project whenever the playlist has any clips, and its comment says "until changes are tracked".

Facts that shape any undo design:

- The engine owns the state and assigns clip ids, so an undo kept only in Redux would drift from the engine.
- `project::capture` and `project::apply` (`project.rs:237`, `:298`) can snapshot and restore the whole project, but `apply` stops playback, re-decodes every sample and gives every clip a new id. It is too heavy to use per undo step as it stands.
- A clip move is one engine call on release. Clip-setting knobs and mixer faders send many throttled updates per drag. The tempo field sends one update per keystroke.
- Deleting a selection is one engine call per clip, so it is not atomic today.

---

## 7. Keyboard shortcuts

There is no hotkey library. Shortcuts come from three places: the native menu, window `keydown` listeners, and per-element `onKeyDown` handlers.

### 7.1 Everything bound today

| Keys | Action | Scope | Where |
|---|---|---|---|
| Cmd/Ctrl+N | New Project | Native menu, global | `engine/src/app/window.rs:79` |
| Cmd/Ctrl+O | Open… | Native menu, global | `window.rs:80` |
| Cmd/Ctrl+S | Save | Native menu, global | `window.rs:83` |
| Cmd/Ctrl+Shift+S | Save As… | Native menu, global | `window.rs:84` |
| Cmd/Ctrl+Shift+M | Open the Mixer tab | Native menu, global | `window.rs:92` |
| Cmd/Ctrl+Shift+E | Open the Sequencer tab | Native menu, global | `window.rs:94` |
| Delete / Backspace | Delete the selected clips | Playlist is the focused view, not typing | `PlaylistCanvas.tsx:172-183` |
| Shift+E | Explode the selected pattern clips | Playlist is the focused view, not typing | `PlaylistCanvas.tsx:186-197` |
| Escape | Clear the selection, cancel the lasso | Anywhere (no guard) | `sketch/index.ts:1136-1141` |
| Escape | Close the clip menu; cancel a sidebar drag | While open / dragging | `ClipMenu.tsx:44-55`, `sampleDrag.ts:111-113` |
| Enter / Escape | Commit / cancel a text field | That field | rename fields, tempo-like fields |
| Space / Enter | Toggle a step | Focused sequencer step cell | `SequencerRow.tsx:85-90` |
| Space / Enter (hold) | Rewind / fast-forward | Focused wind key | `TransportDeck.tsx:156-164` |
| Space / Enter | Preview a sample or take | Focused sidebar row | `SampleItem.tsx:35-36`, `RecordingsBrowser.tsx:62-63` |
| Arrows, PageUp/Down, Home, End | Move a fader or knob | Focused control | `Fader.tsx:72-90`, `Knob.tsx:90-108` |
| Space | "Click" a focused `<button>` | Browser default | — |

### 7.2 What the planned shortcuts collide with

| Planned | Collision today |
|---|---|
| Space (play/stop) | Not bound globally. It does act on focused step cells, wind keys, sidebar rows and any focused button. After clicking Play with the mouse, the Play button keeps focus, so a global Space handler would fire together with the browser's own "click the focused button" unless it prevents the default. |
| C (cut tool), S (select tool) | Free as single keys. Cmd+S is Save and Cmd+C is planned as Copy, so the handlers must ignore modified presses. |
| Shift+J (join) | Free. |
| Cmd+C / Cmd+V / Cmd+X | Not bound. There is **no Edit menu** (the app replaces the default menu with Dawesome / File / Window / Preferences, `window.rs:102-107`). On macOS, text fields normally rely on Edit-menu items for copy and paste. Whether Cmd+C/V/X currently work inside the app's text fields is untested. |
| Cmd+R (repeat) | Not bound and not blocked. Whether the webview reloads the page on Cmd+R depends on the platform webview and is untested. |
| Cmd+Z (if undo is built) | Not bound. |

### 7.3 Supporting facts

- "Is the user typing?" is `isTypingTarget` (`src/util/keys.ts:3-14`): true for text-like `<input>`s (including the number-type tempo field), `<textarea>`, `<select>` and content-editable elements. It is used only by the two playlist handlers.
- "Which view is focused?" is `selectFocusedView` (`src/state/slices/windowSlice.ts:184-185`): the active tab of the pane last clicked in. Clicking the sidebar does not change it.
- Cmd versus Ctrl: the native menu uses `CmdOrCtrl`. The webview code has no platform helper; existing checks treat `metaKey` and `ctrlKey` alike.
- There is no "typing keyboard as piano" feature. No letter key is bound except Shift+E.
- There is no clipboard code of any kind.

---

## 8. Tests and checks

| Check | Command | State on 2026-11-10 |
|---|---|---|
| Frontend tests (Jest 27, ts-jest, jsdom) | `pnpm ui:test` | **178 passed** in 20 suites |
| Engine tests | `cd engine && cargo test` | **194 passed, 5 ignored** (the ignored ones need audio devices; run them with `cargo test -- --ignored`) |
| Typecheck | `pnpm exec tsc --noEmit` (no script exists) | One error: `TS2688: Cannot find type definition file for '@testing-library/jest-dom'` |
| Lint | `pnpm exec eslint src --ext .ts,.tsx` | 58 problems (3 errors, 55 warnings) |
| CI | `.github/workflows/ci.yaml` | Runs only `pnpm ui:test` and `cargo test`, on Ubuntu |

What the tests are:

- **Frontend:** pure logic, reducers and thunks with `invoke` mocked. `src/views/playlist/sketch/selection.spec.ts` shows the pattern for canvas logic: the maths is kept in a p5-free module and tested on its own.
- **No component, canvas or end-to-end tests exist.** React Testing Library is installed but unused, and component rendering does not work under the current Jest setup (`Cassette.spec.ts:1-2` explains why). `jest.config.ts` maps style imports to `jest-transform-css`, which is not installed.
- **Engine:** inline `#[cfg(test)]` modules. They build an engine without hardware with `Engine::new(RATE)`, render buffers and assert on exact frames (`engine.rs:728-869`; `render_run` in `audiograph.rs:1250-1261`). Test WAVs are written with `hound` to the temp folder. Tests rely on the working directory being `engine/` (`"assets/..."` paths).
- No test covers trim, offset, length or stretch, because none of those exist.

---

## 9. Things that already exist and can be reused

- Offline rendering primitives: `Engine::new`, `Renderer::render`, `AudioNode::source()`.
- WAV writing with `hound` (32-bit float, as recorded takes use).
- Project folder conventions, relative paths, unsaved-session folder, free-name helpers, Save As copying, OS trash.
- A p5-free, tested home for selection maths (`sketch/selection.ts`).
- Focus and typing guards for shortcuts (`selectFocusedView`, `isTypingTarget`).
- `ClipMenu` and its item type (`label`, `shortcut`, `onSelect`, `danger`, `disabled`, `title`).
- `LatchKey`, `AnalogButton` (`latched`), `Led`, `ft-tray` for a tool switcher.
- `ProjectNotice` for messages and errors.
- An existing precedent for "split one clip into several": `AudioGraph::explode_pattern_node` (`audiograph.rs:810-849`).

---

## 10. Where the code contradicts or complicates the brief

Each item says what the brief assumed and what the code does. All of them become questions in `01-design-questions.md`.

1. **"Today I can place clips."** You can also move clips (one or a selection, across tracks), delete them, and change per-clip gain, pan, pitch and fades in the Sample tab.
2. **"Instances share the same source."** There is no source object to share. Clips only have equal file paths, and each loads its own copy of the file. "Same source" has to be defined as "same file path".
3. **Undo/redo.** The brief asks how undo works and asks for undo granularity for every operation. Undo does not exist at all, so this project would have to build it from nothing, or go without it.
4. **The playlist is 16 beats (4 bars) long and every clip start is clamped into it.** The brief's own acceptance example ("a 4-bar clip") fills the entire playlist. Paste and Repeat will hit the end almost immediately.
5. **Tempo.** The engine stores clip starts in seconds and the frontend stores them in beats. Changing the tempo updates neither, so after a tempo change the canvas shows a clip at its old beat while the engine plays it at its old time. They agree again only after a save and reload. (Found by reading `set_playlist_tempo`, `daw.rs:511-521`, and `TempoInput.tsx`; I did not reproduce it in the running app.) Trim and split positions need a unit, so this has to be decided.
6. **Pitch already changes speed.** "Time-stretch" with Shift+drag overlaps with what the Pitch knob does today. There is no pitch-preserving stretch and no library for one.
7. **Shift is already the selection modifier.** Shift+click on a clip toggles its selection, and Shift+drag on empty space is the lasso. "Shift+drag an edge = stretch" and "start a lasso over a clip" both land on a Shift-press over a clip and need a rule to tell them apart.
8. **"Shift mode → selection mode"** describes something that partly exists: holding Shift is how selection works now, and Shift+E is already a selection shortcut.
9. **The lasso note is correct but incomplete.** The lasso cannot start over a clip, and it also cannot start at all without Shift, because a plain drag pans.
10. **A plain click does not select a clip.** Selecting one clip to copy currently needs Shift+click.
11. **Space** is used by several focused controls, and there are three play controls with different behaviour (play/pause that keeps the playhead, stop that rewinds, and pattern looping in the Sequencer). "Starts and stops the timeline" does not say which.
12. **No Edit menu**, which matters for Cmd+C/V/X in text fields on macOS.
13. **Deep copy "identical to the original"** has two obstacles in the current playback path. Playback decodes every file to 16 bits, so a bounced file is re-quantised when played. And a clip with gain above 0 dB can produce samples above full scale, which a file played back through a 16-bit decoder would clip.
14. **Deep copy "never writes outside the project folder"** conflicts with how unsaved projects already handle recordings (they go to the app data folder until the first save).
15. **Clips have no name or colour**, so "what each half inherits (name, color)" has nothing to inherit yet. Labels come from the file name.
16. **Pattern clips** have no length, settings or audio of their own. Trim, split, stretch, deep copy and join each need a separate definition for them, or need to be limited to sample clips.
17. **Edits are heard the next time a clip starts.** "Live audio preview while dragging" has no existing mechanism.
18. **The context menu and cursors are not skeuomorphic today** (plain white popover, standard cursors).
19. **Testing.** There is no way to test canvas gestures automatically today, and Tauri's WebDriver tool does not support macOS. Acceptance items phrased as mouse gestures can be checked by unit tests on extracted logic, by engine numeric tests, and by hand.
20. **Commits.** A standing instruction from earlier sessions says I never commit and you review each change. The brief says to commit at the end of each milestone.

---

## 11. What was verified and how

- **Read directly by me:** `clip.rs`, `audiograph.rs` (non-test part), `daw.rs`, `engine.rs` (to line 510), `state/playlist.rs`, `state/state.rs`, `event_dispatch.rs`, `project.rs` (to line 620), `project_files.rs` (to line 215), `recording.rs` (lines 140–310), `main.rs`, `app/window.rs`, `tauri.conf.json`, `capabilities/default.json`, `playlistSlice.ts`, `sketch/index.ts`, `PlaylistObject.ts`, `Waveform.ts`, `selection.ts`, `PlaylistCanvas.tsx`, `ClipMenu.tsx`, `types/clip.ts`, `types/playlist.ts`, `util/keys.ts`, `ci.yaml`, `jest.config.ts`.
- **Reported by exploration agents and spot-checked:** the shortcut table, the list of text fields, the test inventory, the mixer and sequencer internals, the input (recording) writer.
- **Checked in rodio's source:** the decoder's 16-bit output, `Speed` relabelling the sample rate, linear-interpolation rate conversion.
- **Run:** `pnpm ui:test`, `cargo test`, `tsc --noEmit`, `eslint src`.
- **Not verified:**
  - Whether Cmd+C/V/X work in text fields today, and whether Cmd+R reloads the webview. Both need the running app.
  - The tempo mismatch in item 5 of section 10 was found by reading, not reproduced.
  - Whether the engine compiles on Linux. `AudioNode` is gated with `#[cfg(not(target_os = "linux"))]` (`audiograph.rs:173`, `:225`) while ungated code uses it, which suggests it does not, yet CI runs `cargo test` on Ubuntu. The `gh` tool is not installed here, so I could not look at CI results.
  - Whether any time-stretch crate builds with rustc 1.86.
