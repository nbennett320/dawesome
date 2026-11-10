# Later — found during the Playlist editing project, out of its scope

Things worth fixing that this project does not touch unless a design answer brings them in. Each entry says where it is and why it matters.

## Found in Phase 0 (2026-11-10)

- **Playback is 16-bit.** rodio 0.19's decoder outputs `i16`, so every file, including the app's own 32-bit float takes, is reduced to 16 bits on playback (`engine/src/daw/daw_core/sound_buffer.rs:33-39`).
- **Decoding runs on the audio thread.** Each voice decodes lazily as the callback pulls samples (`audiograph.rs:298-300`). Unusual WAV formats panic inside rodio's WAV decoder.
- **Every clip decodes its file twice when added and holds its own copy of the bytes** (`AudioNode::new`, `audiograph.rs:227-265`).
- **Pitch quality.** Linear-interpolation rate conversion with no filter, and the pitched rate is truncated to whole Hz (rodio `Speed`).
- **Clips are not cut at the loop point**, so a clip crossing beat 16 keeps ringing after the jump back.
- **No "unsaved changes" tracking.** `okToReplace` asks whenever the playlist has clips (`src/state/slices/projectSlice.ts:99-108`). Quit calls `process::exit(0)` with no check and has no Cmd+Q accelerator (`engine/src/app/window.rs:30-31`, `:73`).
- **Escape clears the playlist selection from anywhere**, including while typing in a rename field (`src/views/playlist/sketch/index.ts:1136-1141`).
- **The renderer's copy of the snap setting can be stale.** It refreshes at setup and when a drag starts, not when the Snap key is pressed, so ruler seeking can snap to the old setting (`sketch/index.ts:387-394`).
- **Deleting a selection is one engine call per clip** and can fail halfway (`playlistSlice.ts:533-545`).
- ~~**The tempo field sends to the engine on every keystroke.**~~ Changed in M2: a typed tempo is now taken up half a second after typing stops, on Enter, or when the field is left (`src/components/topbar/TempoInput.tsx`).
- **Lint and typecheck baselines are not clean.** `eslint src` reports 3 errors and 55 warnings; `tsc --noEmit` reports a missing type definition for `@testing-library/jest-dom`; Prettier has no config; CI runs neither.
- **Jest cannot render components.** `jest-transform-css` is referenced but not installed, and React's default import is undefined under the current ts-jest settings.
- **Linux build.** `AudioNode` is gated off on Linux while ungated code uses it (`audiograph.rs:173`, `:225`). Not verified against CI.
- **Production bundles may not find samples.** Assets are read by working-directory-relative paths and `bundle.resources` is empty (`engine/tauri.conf.json`).
- **Dead code:** `src/views/sample-details/`, `src/types/documentModel.ts`, `engine/src/lang/` (not compiled), `Track` in `audiograph.rs`, `SoundSource` in `sound_buffer.rs` (its `decoder` calls itself), `buffer.rs`, several unused Tauri commands (`get_playlist_sample_offset`, `get_playlist_max_length`, the time-signature pair), the legacy `pixelOffset` on every clip.
- **README is stale** (old import path, old command names, `backend/` paths).

## Found while building M1 (2026-11-10)

- **A clip's tail past bar 64 cannot be scrolled into view,** and the loop's end can lie past bar 64 for such a clip (`src/views/playlist/sketch/index.ts`, the pan limit in `p.draw`). The same was true past bar 4 before.
- **With Loop on, seeking past the loop's end and pressing Play starts from the beginning** (`start_playlist_after`, `engine/src/daw/daw.rs`). The rule is old; with a 64-bar ruler it is easier to run into.
- **Mouse-wheel zoom is capped at 3.5% per notch** (`staticDefaults.zoomSensitivity`), so zooming from the opening view out to the whole playlist takes about 80 notches.
- **The loop's bar length comes from the project's time signature, the canvas grid assumes 4 beats to the bar** (`theme.beatsPerBar`). They differ only for a hand-edited project file.
- **For up to a beat after the loop is shortened behind the playhead, the engine's own playhead reading is wrong** (it is right again once the scheduler wraps). A Pause in that window parks the playhead in the wrong place.

## Found while building M5 and M6 (2026-11-10)

- **A failed Play/Pause is not caught** (`togglePlay`, `src/state/slices/playlistSlice.ts`): if the engine refuses, the error goes nowhere. Same for the topbar key and Space.
- **A right-button release during a left-button press ends the press** (`canvas.mouseReleased`, `src/views/playlist/sketch/index.ts`, does not check which button came up).
- **`src/components/common/base/SelectBase.tsx` looks unused.** If it comes back into use, Space on it would play instead of opening it.

## Found while building M3 (2026-11-10)

- **Undo pressed while a dropped sample is still loading undoes the edit before the drop** (`add_clip`, `engine/src/event_dispatch.rs`, records its step only after the file is decoded).
- **The Edit menu's Undo and Redo items are never dimmed.** The engine already returns `canUndo` and `canRedo` with every undo and redo.
- **`set_playlist_time_signature` records no undo step.** Nothing in the UI calls it.

## Found while building M4 (2026-11-10)

- **Each new clip's whole-file waveform is fetched again** by the canvas, including the second half of a cut (`fetchNodeData`, `src/views/playlist/sketch/index.ts`).
- **`ClipSettings::apply`, `Envelope` and `Pan` in `engine/src/daw/daw_core/clip.rs` are now only used by tests,** as the reference the new reader is compared against.
- **`AudioNode.duration` (a sample's length) is worked out in single precision,** so for a long file it can be off by tens of microseconds (`duration_and_waveform`, `audiograph.rs`). Windows work around it by treating "to the end" as its own case.
- **rodio's `size_hint` can overflow** on a converted stereo source (it panics in debug builds if something collects such a source into a `Vec`). Nothing in the app does.
- **A pitched file is played at a whole number of samples per second** (`pitched_rate`, `engine/src/daw/daw_core/clip_reader.rs`, as rodio's `Speed` did), but lengths and positions are worked out from the exact pitch. A clip's sound and its drawn length can differ by up to about 1 ms per 10 s. Working everything out from the whole-number rate, in the engine and on the canvas, would close the gap.
- **Raising a trimmed clip's pitch in the Sample tab can make it shorter than the 10 ms a trim allows** (`AudioNode::set_settings`). Nothing breaks.
- **`set_clip_windows` and `split_clips` check ids before changing anything, but not that every pattern clip's pattern still exists.** A pattern's clips are removed with it, so that should not happen.
- **Pressing an edge reads the mouse position p5 last saw,** which is one event old (`edgeUnderMouse`, `src/views/playlist/sketch/index.ts`). A press with no mouse movement before it could judge the zone from the wrong place. The lasso reads the event itself.
- **Starting a second edge drag before the engine has answered the first** (a few milliseconds) measures the clip from where it was before the first trim.

## Found while building M11 to M13 (2026-11-10)

- **A crash while a copy or a join is being written leaves a hidden `.<name>.part` file in `clips/`** (`write_wav`, `engine/src/bounce.rs`). It is harmless and is not cleaned up.
- **A clip file whose name looks like a take's (`<project>_<date>_<n>.wav`) is renamed like a take on Save As** (`relocate_takes`, `engine/src/project_files.rs`). Deep copy and Join never make such names.
- **A paste from the empty-space menu is not held to the track limit a move is** (`paste`, `engine/src/clipboard.rs`).
- **The folder helpers in `engine/src/project_files.rs` say "recording" in their refusals;** the Clips page swaps the word in the finished sentence (`Library::worded`, `engine/src/audio_commands.rs`), which also changes a quoted file name that contains it. Passing the noun in would be cleaner.
- **The fps readout has no switch:** it is `debugMode` in `staticDefaults`, `src/views/playlist/sketch/index.ts`.
