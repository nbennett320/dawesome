# 03 — Plan

Written 2026-11-10. Milestones are in dependency order. Each ends with working, tested code, an updated `04-acceptance.md`, a fresh-eyes verifier pass, and one commit. Tick the `Approved` box under each milestone you accept. Section numbers (§) refer to `02-spec.md`.

**For every milestone**

- Test commands: `pnpm ui:test` from the repo root, and `cargo test` from `engine/`.
- Gate: both suites pass, and `pnpm exec tsc --noEmit` and `pnpm exec eslint src --ext .ts,.tsx` report no problems beyond today's baseline (one tsc error; 3 eslint errors and 55 warnings).
- Canvas logic goes in p5-free modules beside `src/views/playlist/sketch/selection.ts`, with unit tests, as that file does today.
- Audio behaviour is proved by engine tests that compare rendered sample buffers.
- You run the milestone's manual checklist from `04-acceptance.md` in the app.

**Overview**

| # | Milestone | Depends on | Size |
|---|---|---|---|
| M1 | 64-bar playlist and loop | — | Medium |
| M2 | Tempo keeps clips on their beats | M1 | Medium |
| M3 | Undo and redo for existing operations, Edit menu | M2 | Large |
| M4 | Clip windows in the engine | M3 | Large |
| M5 | Space | — | Small |
| M6 | Lasso over a clip | — | Small |
| M7 | Trim | M4 | Large |
| M8 | Stretch | M7 | Medium |
| M9 | Playlist tray, tools, Cut | M7 | Medium |
| M10 | Clipboard, Repeat, Select All, context menus | M4, M3 | Medium |
| M11 | Deep copy | M4, M10 | Large |
| M12 | Clips sidebar page | M11 | Medium |
| M13 | Join | M11, M9 | Large |
| M14 | Performance and full acceptance run | all | Small |

M5 and M6 depend on nothing and could be handed to a cheaper model at any time.

---

## M1 — 64-bar playlist and loop

**Goal:** the playlist is 64 bars; the loop ends at the last clip; the view opens on the first 4 bars (§2.1, §2.2).

**Work**

- Engine: playlist length becomes 64 bars (256 beats) everywhere it is used for clamping and seeking. The loop length becomes "end of the last clip, rounded up to a bar; 4 bars when empty". The loop's end is frozen for the length of a recording. The transport report gains the loop's end.
- Frontend: the canvas maps beats to pixels from a "beats visible at zoom 1" value of 16 instead of the playlist length, and allows zooming out to all 256 beats. Beat lines and ruler numbers thin out when zoomed out. A loop-end marker is drawn on the ruler while Loop is on. New Project and Open reset the view.

**Files**

- `engine/src/daw/state/defaults.rs`, `engine/src/daw/state/playlist.rs` (`loop_len`), `engine/src/daw/daw.rs` (`start_playlist_after`, `run_playlist`, `seek_playlist`), `engine/src/event_dispatch.rs` (`beat_to_offset`, `PlaylistTransport`, `get_playlist_data`), `engine/src/recording.rs`
- `src/views/playlist/sketch/index.ts` (zoom limits, grid), `Timeline.ts` (ruler numbers, loop marker), `theme.ts`, `src/types/transport.ts`, `src/views/transport/tape.ts`, `src/state/slices/playlistSlice.ts`

**Risks**

- Several places assume "zoom 1 shows the whole playlist" (`#zoomBy` keeps scale at 1 or more; `beatToX` divides by `beatsDisplayed`). Minimum zoom has to become a fraction.
- The Transport tab's tape maths reads `loopBeats` and `beatsDisplayed` (`tape.ts`); it must keep working with the new values.
- Recording uses the loop length to end takes (`recording.rs`).

**Tests**

- Engine: loop length for an empty playlist, for clips ending mid-bar, and frozen during a recording; a clip can be placed and moved to bar 60; seek clamps at bar 64.
- Frontend: grid-thinning function (which lines and numbers are drawn at a given pixels-per-beat); beat-to-pixel maths at fractional zoom; existing `tape.spec.ts` still passes.

- [ ] Approved

---

## M2 — Tempo keeps clips on their beats

**Goal:** clips and the playhead stay on their beats when the tempo changes; a live change carries on at the new tempo; tempo is locked while recording (§2.3).

**Work**

- Engine: clip and playhead positions are held in beats, with time derived from the tempo when scheduling. Setting the tempo while playing ends the run and starts a new one from the same beat. Setting the tempo while recording is refused.
- Frontend: the tempo field is disabled while recording.

**Files**

- `engine/src/daw/daw_core/audiograph.rs` (`AudioNode`, `PatternNode`, `slice_sounds`, `move_*`, `group_move`, `end`), `engine/src/daw/daw.rs` (`set_playlist_tempo`, playhead), `engine/src/event_dispatch.rs`, `engine/src/project.rs` (`capture`, `apply`), `engine/src/recording.rs`
- `src/components/topbar/TempoInput.tsx`, `src/state/slices/playlistSlice.ts`

**Risks**

- This changes the unit of `start_offset`, which most of `audiograph.rs` and its 37 tests use. I will keep a derived time accessor so the scheduling code changes as little as possible.
- Snapping is done in microseconds of time today; it becomes snapping in beats, and the existing snap tests must give the same answers.

**Tests**

- Engine: after a tempo change, a clip at beat 8 reports beat 8 and renders its first frame at the new beat-8 frame; the playhead reports the same beat; save and reload give the same positions without rounding drift; tempo change during a faked recording is refused.

- [ ] Approved

---

## M3 — Undo and redo for existing operations, Edit menu

**Goal:** whole-project undo and redo for everything that exists today, driven from the engine, with Cmd+Z, Shift+Cmd+Z and an Edit menu (§4, §5).

**Work**

- Engine: a history of up to 100 steps. Each undoable command records how to reverse and re-apply itself. Commands that today run once per clip (delete a selection) get a single batched form. Gesture grouping: the frontend marks the start and end of a knob, fader, step-painting or tempo-field gesture, and the engine merges what happens between them into one step. `undo` and `redo` commands return what changed so the frontend can redraw and select the affected clips. The history clears on New and Open and is blocked during a recording.
- Native menu: an Edit menu with Undo, Redo, Cut, Copy, Paste, Repeat, Select All, Join. In this milestone only Undo and Redo do anything for the project; the clipboard items work in text fields (the standard macOS behaviour) and are wired to clips in M10 and M13.
- Frontend: key handling for Cmd+Z and Shift+Cmd+Z outside text fields; redraw after undo for playlist, mixer, sequencer and tempo; selection set to the affected clips.

**Files**

- New: `engine/src/history.rs` (history and step types), `engine/src/history_commands.rs`
- `engine/src/event_dispatch.rs`, `engine/src/sequencer_commands.rs`, `engine/src/mixer_commands.rs`, `engine/src/daw/daw_core/audiograph.rs`, `mixer.rs`, `sequencer.rs`, `engine/src/daw/state/state.rs`, `engine/src/project.rs` (clear on load), `engine/src/app/window.rs`, `engine/src/main.rs`
- `src/events/window.ts`, `src/util/keys.ts`, `src/state/slices/playlistSlice.ts`, `mixerSlice.ts`, `sequencerSlice.ts`, new `src/state/slices/historySlice.ts`; gesture start and end calls in `src/views/mixer/Fader.tsx`, `src/components/fourtrack/Knob.tsx`, `src/views/sample-preview/SettingControl.tsx`, `src/views/sequencer/SequencerRow.tsx`, `src/components/topbar/TempoInput.tsx`, `src/views/playlist/PlaylistCanvas.tsx`

**Risks**

- This is the widest milestone: it touches every command that changes project state. To keep it reviewable I will land it in three commits inside the milestone if you prefer (playlist, then mixer and tempo, then sequencer); say so when approving.
- A deleted clip must come back with its original id so that later steps still refer to it. Deleted sequencer rows and patterns need the same.
- Whether a native Edit menu's Cut, Copy and Paste items reach the webview as `cut`, `copy` and `paste` events is unverified. The first task of this milestone is a ten-minute check in the running app. If they do not, the items send menu events like the File menu does, and text fields are handled with the predefined native items.
- Whether Cmd+Z in a text field undoes typing once a native Undo item exists is the same check.

**Tests**

- Engine: for each undoable command, apply, undo, and compare the captured project to the one before; redo and compare to the one after. A 150-step run keeps exactly 100. A new edit after undo clears redo. Gesture grouping: 20 fader updates between gesture marks undo as one step. Deleted clips, rows and patterns come back with their ids. History is empty after `new_project` and `load_project`. Undo is refused while a recording is faked.
- Frontend: reducers for the undo result; the key predicate (Cmd on macOS, Ctrl elsewhere, ignored in text fields).

- [ ] Approved

---

## M4 — Clip windows in the engine

**Goal:** sample clips and pattern clips can play a window of their source; the project file stores it; nothing in the UI uses it yet (§3, §8.3, §8.5, §15, the engine half of §10.3).

**Work**

- Engine: each clip gains a window (start within the source, length). A clip reader produces the windowed audio through the existing settings chain, and keeps the resampling phase the same wherever the window starts, so two neighbouring windows produce exactly the frames the whole clip does. Fades are measured from the window's edges. Pattern clips schedule only hits that start inside their window, remember their original window, and clamp to the pattern's current length. Commands: set a clip's window (one or many clips), split a clip. Both are undoable. Waveform data for a window is served so the canvas can draw it.
- Project file: window fields on `ProjectClip` and `ProjectPatternClip`, omitted when a clip is whole; version 3 written only when some clip has a window.

**Files**

- `engine/src/daw/daw_core/audiograph.rs`, `clip.rs`, new `engine/src/daw/daw_core/clip_reader.rs`, `sequencer.rs`, `engine/src/event_dispatch.rs`, `engine/src/project.rs`, `engine/src/history.rs`, `engine/src/main.rs`
- `src/types/playlist.ts`, `src/types/clip.ts`, `src/types/project.ts`, `src/state/slices/playlistSlice.ts` (types and reducers only)

**Risks**

- "Identical at any pitch and rate" (F35) rules out rodio's `skip_duration` and its sample-rate converter for windowed clips, because both restart the interpolation at the window's start. The reader has to do its own linear interpolation with a phase carried from the source's first sample. It must still match today's sound for whole clips; a test compares the two.
- rodio's `Speed` truncates the pitched rate to whole Hz. The reader must reproduce that or deliberately replace it; either way whole clips must sound as they do now, within the 16-bit tolerance.
- Resuming mid-clip (play from a paused position) uses the same reader, so it must also be phase-correct.

**Tests (numeric)**

- A whole clip through the new reader equals the current path's output, frame for frame, at pitch 0 and file rate equal to engine rate; within one 16-bit step otherwise.
- Split at an arbitrary frame: left half followed by right half equals the uncut clip, every frame, at pitch −7, 0 and +5 and at 44.1 kHz and 48 kHz files on a 48 kHz engine.
- A window's fade-in starts at the window's start; a fade longer than the window is clamped.
- Pattern clip window: only hits starting inside it are scheduled; shortening the pattern clamps it; lengthening restores it; a window past the end plays nothing.
- Project: a trimmed clip round-trips; a project with no windows is still written as version 1 or 2; a version 2 fixture loads with whole clips.
- Undo and redo of set-window and split.

- [ ] Approved

---

## M5 — Space

**Goal:** Space is the Play/Pause key everywhere except while typing (§17).

**Work:** one window-level key handler that calls the same action as the topbar Play/Pause key, ignores key repeat, ignores typing targets, and prevents the default so a focused button is not also "clicked". Remove Space from the per-control handlers that use it (they keep Enter).

**Files:** `src/util/keys.ts`, `src/App.tsx` or a small new hook, `src/views/sequencer/SequencerRow.tsx`, `src/views/transport/TransportDeck.tsx`, `src/components/toolbars/sample-browser/SampleItem.tsx`, `src/components/toolbars/recordings-browser/RecordingsBrowser.tsx`

**Risks:** react-tabs and other focusable controls may have their own Space handling; the handler listens in the capture phase so it wins.

**Tests:** key predicate unit tests (Space with and without modifiers, repeat, each typing target type); a thunk test that Space dispatches the same action as the Play/Pause key in Playlist and Sequencer focus.

- [ ] Approved

---

## M6 — Lasso over a clip

**Goal:** Shift+drag starts a lasso anywhere after 10 px; Shift+click still toggles (§7, without the edge-zone exception, which arrives with M8).

**Work:** a Shift-press records a pending gesture instead of acting at once. Movement past the threshold starts the lasso from the press point; release without movement toggles the clip under the press, or does nothing on empty space.

**Files:** `src/views/playlist/sketch/index.ts`, `src/views/playlist/sketch/selection.ts` (press-resolution function), `selection.spec.ts`

**Risks:** the lasso currently exists from the moment of the press; drawing and auto-scroll must wait for the threshold.

**Tests:** press-resolution function: press on clip with no movement toggles; movement of 9 px does not start a lasso, 11 px does; a lasso started on a clip includes it; the result always adds to the existing selection.

- [ ] Approved

---

## M7 — Trim

**Goal:** edge drags trim sample and pattern clips, with snapping, multi-selection, triangles, the hover bar and the Sample tab highlight (§8).

**Work**

- Canvas: edge hit zones; hover bar and cursor; a live trim drag that redraws the clip from its windowed waveform; snap to grid and to the playhead; Option bypass; limits; multi-selection deltas; Escape cancel; apply on release even outside the canvas; triangles on trimmed edges; pattern clips draw only their window.
- One engine call on release (the set-window command from M4), as one undo step.
- Sample tab: whole-file waveform with the played part highlighted, envelope inside it.

**Files**

- `src/views/playlist/sketch/index.ts`, `PlaylistObject.ts`, `Waveform.ts`, `theme.ts`, new `src/views/playlist/sketch/edges.ts` (hit zones, snap, limits, multi-trim maths) with `edges.spec.ts`
- `src/views/playlist/PlaylistCanvas.tsx`, `src/state/slices/playlistSlice.ts`
- `src/views/sample-preview/ClipWaveform.tsx`, `SamplePreview.tsx`
- `engine/src/event_dispatch.rs` (waveform and clip info for windows, if not complete in M4)

**Risks**

- The waveform is drawn from levels for the whole played clip. A live trim needs the whole file's levels and a visible range, so `Waveform` changes how it maps points to pixels.
- Releasing outside the canvas currently cancels every drag in the window-level handler; edge drags need their own path there.
- The playhead-snap rule needs the playhead beat on the renderer while stopped; it is already fetched for the cursor.

**Tests**

- `edges.spec.ts`: zone widths at several clip widths (including the third-of-width rule); snap to grid; playhead wins only when nearer than both grid lines, and only when stopped with Snap on; Option bypass; left and right limits; 10 ms and one-step minimums; multi-trim where one clip hits its limit and the others continue.
- Engine: after a trim to bar 2, the clip's last rendered frame is the same source sample that was at bar 2 before.
- Reducers for the trimmed clip state.

- [ ] Approved

---

## M8 — Stretch

**Goal:** Shift+dragging an edge stretches by writing Pitch (§9), and a Shift-press in an edge zone is told apart from a lasso or a toggle (§7).

**Work:** the edge drag from M7 gains a stretch mode fixed at the press. The target length gives a pitch value, limited to ±24 semitones. With a left-edge stretch the start moves and the end stays. Multi-selection applies the same ratio. One engine call on release sets pitch (and start, for a left-edge stretch) for every affected clip as one undo step.

**Files:** `src/views/playlist/sketch/index.ts`, `edges.ts`, `edges.spec.ts`, `src/state/slices/playlistSlice.ts`, `engine/src/event_dispatch.rs` (a batched "set settings and placement" command), `engine/src/history.rs`

**Risks**

- Pitch is stored as a 32-bit float and the engine truncates the pitched rate to whole Hz, so a clip stretched "to bar 3" can end a fraction of a millisecond away. The acceptance item states the tolerance (one output frame).
- A stretched clip's fades are in milliseconds and are clamped when it gets shorter (existing rule).

**Tests:** `edges.spec.ts`: length-to-pitch and back; the ±24 limit stops the edge; left-edge stretch keeps the end; same ratio across a selection with one clip at its limit; pattern clips are skipped. Engine: a clip stretched to twice its length renders twice as many frames (within one frame) and reports pitch −12.

- [ ] Approved

---

## M9 — Playlist tray, tools, Cut

**Goal:** the tray above the ruler with Select and Cut keys and the snap controls; C and S; the Cut tool; "Split at playhead" (§6, §10).

**Work**

- Tray component inside the Playlist pane; Snap key and snap-value drop-down moved out of the topbar; two new icons; the razor-blade cursor (SVG, shown to you before it ships).
- Tool state on the renderer and in Redux (for the keys' LEDs); C and S handlers.
- Cut tool: hover line at the snapped position, release-to-cut, multi-clip rule, minimum-length rule. "Split at playhead" menu item. Both call the split command from M4 as one undo step.

**Files**

- New: `src/views/playlist/PlaylistTray.tsx`, `src/components/icons/SelectToolIcon.tsx`, `CutToolIcon.tsx`, `public/cursors/razor.svg`, `src/views/playlist/sketch/cut.ts` with `cut.spec.ts`
- `src/views/playlist/Playlist.tsx`, `PlaylistCanvas.tsx`, `sketch/index.ts`, `src/components/topbar/ButtonArea.tsx`, `src/state/slices/playlistSlice.ts` or `windowSlice.ts`, `src/util/keys.ts`

**Risks**

- The canvas fills its pane and measures itself with a `ResizeObserver`; the tray takes height from it.
- Custom cursors in the macOS webview need a small image with an explicit hot spot; if it will not render, the crosshair fallback applies and I tell you.

**Tests:** `cut.spec.ts`: cut position with grid, playhead and Option; refusal inside the minimum length; which clips a click cuts given a selection. Key predicate tests for C and S (modifiers, focus, typing). Engine split tests exist from M4; add "Split at playhead" at an unsnapped position.

- [ ] Approved

---

## M10 — Clipboard, Repeat, Select All, context menus

**Goal:** Cmd+C, Cmd+X, Cmd+V, Cmd+R, Cmd+A; the Edit menu items wired to clips; the clip menu's new items and target rule; the empty-space menu (§13, §16, §7 for Cmd+A).

**Work**

- Engine: a batched "add clips" command (copies of given clips at given positions, sharing the file bytes already loaded) and a batched delete, each one undo step. End-of-playlist check that refuses the whole batch.
- Frontend: clipboard state (in memory, cleared on New and Open); paste position rules; Repeat; Select All; the reload block for Cmd+R; menu items, dividers, the selection-target rule; the empty-space menu.

**Files**

- `engine/src/event_dispatch.rs`, `engine/src/daw/daw_core/audiograph.rs`, `engine/src/history.rs`, `engine/src/main.rs`
- New: `src/state/clipboard.ts` with `clipboard.spec.ts` (pure placement maths)
- `src/views/playlist/PlaylistCanvas.tsx`, `ClipMenu.tsx`, `sketch/index.ts`, `src/state/slices/playlistSlice.ts`, `src/events/window.ts`, `src/util/keys.ts`

**Risks**

- Whether the webview reloads on Cmd+R, and how to stop it in development and release builds, is unverified. First task: check in the running app. A native Edit › Repeat item with that accelerator should take the key before the webview sees it.
- Pasted sample clips should reuse the already-loaded file bytes instead of reading the file again, or a 100-clip paste will be slow.

**Tests:** `clipboard.spec.ts`: paste offsets from the playhead with snap on and off; original tracks; refusal when any clip would start past bar 64; Repeat's start equals the span's end exactly; skipped clips for a missing pattern; empty-space paste track and position. Engine: batched add and delete undo as one step; pasted clips are independent of their originals (changing one's window leaves the other's render unchanged).

- [ ] Approved

---

## M11 — Deep copy

**Goal:** "Deep copy" bounces a clip to `clips/` and swaps it in place, for sample and pattern clips, singly or as a selection (§11).

**Work**

- Engine: an offline bounce of one clip through its own settings chain to 32-bit float stereo WAV at the engine rate, clipped at full scale; file naming with free-number search; write to a temporary name and rename; swap the clip to the new file with neutral settings; one undo step that restores the old clip and keeps the file; redo reuses the file. Pattern-clip bounce with a 10 ms tail fade. `clips/` handling for bare-file projects and Save As.
- Frontend: menu item; Save-dialog-first flow for unsaved projects; wait cursor after half a second; errors in the notice banner.

**Files**

- New: `engine/src/bounce.rs`
- `engine/src/project_files.rs` (`clips/` folder, naming, Save As copy), `engine/src/project.rs`, `engine/src/event_dispatch.rs`, `engine/src/history.rs`, `engine/src/main.rs`
- `src/views/playlist/PlaylistCanvas.tsx`, `src/state/slices/playlistSlice.ts`, `src/state/slices/projectSlice.ts`

**Risks**

- The accuracy target depends on the copy playing back with no resampling, so the file must be at the engine's rate and its length must be the exact frame count the original clip renders.
- If the output device changes rate later, old copies are resampled like any other file. That is expected and is not covered by the accuracy test.
- A selection's deep copy must be all-or-nothing: write every file first, then swap, and delete the new files if any write fails.

**Tests (numeric)**

- A clip with trim, pitch −3, a 50 ms fade-in, pan 0.5 and gain −6 dB: render the original, deep copy, render the copy; same frame count; every sample within one 16-bit step.
- A clip with gain +12 dB on a loud file: the copy equals the original clipped to full scale, within one 16-bit step.
- The copy's file is 32-bit float, stereo, at the engine rate, and its frame count equals the clip's rendered frame count.
- Pattern clip: the file equals the rendered pattern clip for its length, with the last 10 ms faded.
- Naming picks the next free number and never overwrites; a path outside the project folder is impossible (the target is always built from the project's folder); an unsaved project is refused at the engine.
- Undo restores the original clip and leaves the file; redo does not write a second file.
- A selection where the second write fails leaves every clip unchanged and no new files.
- Speed: a one-minute clip bounces in under 2 seconds (marked as a timing test, run locally).

- [ ] Approved

---

## M12 — Clips sidebar page

**Goal:** a Clips page that behaves like the Recordings page, pointed at `clips/` (§12).

**Work:** generalise the recordings commands and the Recordings browser so both pages share one implementation that takes the folder as a parameter; add the rail key and its icon (shown to you before it ships).

**Files:** `engine/src/audio_commands.rs`, `engine/src/project_files.rs`, `engine/src/main.rs`, `src/components/toolbars/recordings-browser/RecordingsBrowser.tsx` (shared browser extracted), new `src/components/toolbars/clips-browser/ClipsBrowser.tsx`, `src/components/sidebar/Sidebar.tsx`, `src/types/ui.ts`, `src/types/recordings.ts`, `src/state/slices/windowSlice.ts`, new `src/components/icons/ClipsIcon.tsx`

**Risks:** the recordings commands also drive take naming and the unsaved-session folder, which do not apply to `clips/`; the shared code must keep those paths recordings-only. This is a refactor of working code, so the existing recordings tests are the safety net.

**Tests:** engine: list, rename (clips using the file follow), move, folder, delete (refused while in use) against a temporary `clips/`; all existing recordings tests still pass. Frontend: the existing `recordings.spec.ts` helpers run for both folders.

- [ ] Approved

---

## M13 — Join

**Goal:** Shift+J and the menu's Join, for sample clips (heal or bounce) and pattern clips (new pattern) (§14).

**Work**

- Engine: decide heal or bounce; heal merges windows; bounce renders the selected clips on one track into one file (gaps silent, overlaps summed) using M11's writer and rules. Pattern join builds a new pattern from the clips' windows on the first clip's sixteenth-note grid, rounds its length up to a bar, names and places it, and replaces the clips with one windowed clip. Patterns may be any whole number of bars. Every form is one undo step.
- Frontend: the Shift+J handler, Edit menu and clip menu items, refusals in the notice banner, the joined clip selected. Sequencer: the bar keys show none lit for other lengths.

**Files:** `engine/src/bounce.rs`, `engine/src/daw/daw_core/audiograph.rs`, `sequencer.rs` (`check_bars`, new-pattern-from-steps), `engine/src/sequencer_commands.rs`, `engine/src/event_dispatch.rs`, `engine/src/history.rs`, `engine/src/main.rs`; `src/views/playlist/PlaylistCanvas.tsx`, `src/util/keys.ts`, `src/state/slices/playlistSlice.ts`, `sequencerSlice.ts`, `src/views/sequencer/PatternStrip.tsx`, `src/types/sequencer.ts`

**Risks**

- Lifting the 1 / 2 / 4 bar limit touches the Sequencer grid, which may assume those lengths when laying out steps; I check it draws a 3-bar and an 8-bar pattern before relying on it.
- "Line up" for a heal must mean the result plays identically: same file, same gain, pan and pitch, touching in the source and on the timeline, no fade-out on the left piece and no fade-in on the right. Anything else bounces.

**Tests (numeric where audio)**

- Heal: cut a clip, join the halves; the result renders identically to the original and no file is written.
- Bounce: two different samples with a gap; the joined clip's render equals the sum of the two originals' renders over the span, within one 16-bit step; overlap case likewise.
- Refusals: two tracks; mixed kinds; pattern clips off the sixteenth grid.
- Pattern join: two 1-bar pattern clips a bar apart give a 3-bar pattern with an empty middle bar; the new clip schedules the same hits at the same times as the two originals did; originals untouched; name is "a + b"; undo removes the pattern and restores both clips.

- [ ] Approved

---

## M14 — Performance and full acceptance run

**Goal:** the performance bar holds and every item in `04-acceptance.md` is checked (§20).

**Work:** generate a 500-clip test project; measure frame rate during an edge drag, a cut hover and a lasso with the canvas's built-in readout; time each edit command; fix what misses the bar. Run the whole acceptance checklist, with a verifier subagent checking code against spec for every section.

**Files:** whatever the measurements point at; a script under `docs/editing-project/` that writes the test project.

**Risks:** the canvas rebuilds every clip object when the clip list changes; a 500-clip paste may need the rebuild to be incremental.

**Tests:** engine timing tests for batched add, delete, set-window and split on 500 clips (each under 50 ms).

- [ ] Approved

---

## Unverified assumptions that the plan depends on

Each is checked at the start of the milestone named, before other work, and I come back to you if it fails.

| Assumption | Checked in |
|---|---|
| A native Edit menu's Cut, Copy and Paste reach the webview as clipboard events, and text fields keep working | M3 |
| Cmd+R can be kept from reloading the webview in development and release | M10 |
| A custom SVG cursor renders in the macOS webview over the canvas | M9 |
| The Sequencer grid draws patterns that are not 1, 2 or 4 bars | M13 |
| The 500-clip frame-rate bar is reachable without replacing p5 | M14 (measured informally from M7 on) |
