# Handoff brief — M11 — Deep copy

This brief stands on its own. It was assembled on 2026-11-10 from `02-spec.md`, `03-plan.md` and `04-acceptance.md` in the folder above; if they disagree with this brief, they win.

## Before you start

- Repo: Dawesome, a DAW. Rust audio engine and Tauri 2 app in `engine/` (one crate, rustc 1.86), React 18 + TypeScript + Redux Toolkit frontend in `src/`, the Playlist drawn on one p5.js canvas (`src/views/playlist/sketch/`). The engine owns project state; every frontend edit is a thunk that calls `invoke(...)` and then stores what the engine returned.
- Read `docs/editing-project/lessons.md` first, then `docs/editing-project/00-codebase-map.md` for where things are.
- Work on branch `playlist-enhancements`. Never edit, stage or commit `docs/todo.md`.
- Tests: `pnpm ui:test` from the repo root; `cargo test` from `engine/` (tests open samples by paths relative to `engine/`).
- Gate: both suites pass, and `pnpm exec tsc --noEmit` and `pnpm exec eslint src --ext .ts,.tsx` report nothing beyond the baseline (one tsc error about `@testing-library/jest-dom`; 3 eslint errors and 55 warnings).
- Canvas logic goes in p5-free modules beside `src/views/playlist/sketch/selection.ts` with unit tests. Audio behaviour is proved by engine tests that compare rendered sample buffers.
- Do not add features, refactor, or restyle beyond what the spec excerpts below require. Note anything else worth fixing in `docs/editing-project/later.md`.
- If the spec does not cover a decision, stop and ask the user. `Q…`, `F…`, `G…`, `H…` in the excerpts are the user's recorded answers (`01-answers.md`, `01b-follow-ups.md`).
- Commit once at the end: one short lowercase line in the user's style (for example "caching waveforms"), no trailer lines.

## The milestone (from the plan)

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

## Spec excerpts

### 11. Deep copy

#### 11.1 Command

- "Deep copy" is an item in the clip's context menu, with that exact label and no shortcut (Q122). It is offered on every clip it applies to (Q102), including pattern clips (Q26).
- It replaces the clip in place: the same clip, at the same position and track, now plays the new file (Q103).
- Chosen on a clip inside a selection, it applies to every selected clip. Each gets its own new file. It is one undo step, and if any file fails, no clip is changed (Q150, G2).

#### 11.2 What the file contains

- The clip as it plays: trim, pitch and speed, fades, pan and gain are all baked in. The track's mixer strip and the master are not (Q104).
- Samples above full scale are clipped to full scale in the file (Q105).
- Afterwards the clip's settings are neutral (no trim, Pitch 0, no fades, centre pan, 0 dB), so nothing is applied twice (Q104).
- The file is exactly as long as the clip's played length, with no added silence or tail (Q121).
- Format: WAV, 32-bit float (Q108), always stereo (Q109), at the engine's current output rate, or the source file's rate if no output device is open (Q107).

#### 11.3 Accuracy

Played back, the copy has the same number of frames as the original clip, and every sample is within one 16-bit step (about 0.00003 of full scale) of the original, except samples the original pushes above full scale, which the copy clips (Q106).

#### 11.4 Files

- Files go in `clips/` inside the project folder. For an older project that is a bare `.dawesome` file, `clips/` sits beside that file (Q114).
- Name: `<source name>_copy_<n>.wav` (Q110). The next free number is used, names are compared without regard to case, and no existing file is ever overwritten (Q111).
- The old source file is never deleted or moved (Q112).
- In a project that has never been saved, the Save dialog opens first. The deep copy happens after the save and is abandoned if you cancel (Q113).
- Save As copies `clips/` into the new project folder, and the new project's clips point at the copies (Q115).
- The project is not saved automatically afterwards (Q120).

#### 11.5 Feedback

- If writing takes longer than about half a second, the mouse shows the wait cursor until it finishes. Otherwise nothing is shown (Q117).
- On any failure the clip is left exactly as it was, any partly written file is removed, and the reason is shown in the notice banner in its error style (Q118, F37).
- An edit that lands while the clip is sounding is heard from its next start (Q63).
- A one-minute clip is written in under 2 seconds on your Mac (Q164).

#### 11.6 Undo

Undo puts the clip back on its original file with its original trim and settings. The new file stays on disk. Redo reuses that file and does not write another (Q119).

#### 11.7 Pattern clips

- The clip is rendered as it plays at that moment: current steps, row volumes applied, muted rows silent, at the current tempo (F21).
- The file is exactly the clip's length; any hit still ringing at the end is faded out over 10 ms (F20).
- Name: `<pattern name>_copy_<n>.wav` (F21).
- Afterwards the clip is an ordinary sample clip, no longer linked to the pattern. Undo turns it back into the pattern clip (F21).

### 16. Context menus

#### 16.1 Clip menu

- Items, in order: Cut (⌘X), Copy (⌘C), Paste (⌘V), Repeat (⌘R); divider; Split at playhead, Join (⇧J), Deep copy; divider; Delete (⌫). A pattern clip keeps "Explode into tracks" and "Open in Sequencer" at the top (Q148, Q149).
- Items that do not apply are left out: Join with fewer than two clips selected (Q148), Split at playhead when the playhead does not cross the clip (F31), Paste when the clipboard is empty (F38).
- Paste from a clip's menu pastes at the playhead on the original tracks, like Cmd+V (Q152a).
- Right-clicking a clip that is part of a selection applies every item to the whole selection, Delete included (Q150).
- Right-clicking a clip that is not selected leaves the selection unchanged and applies the items to that clip only (Q151).
- The menu keeps its current look. Only items and divider lines are added (Q153).

#### 16.2 Empty-space menu

- Right-clicking empty space opens a menu with Paste (Q152). With an empty clipboard no menu opens (F38).
- That Paste puts the earliest copied clip at the right-clicked position, snapped when Snap is on. The topmost copied clip goes on the right-clicked track and the others keep their spacing in time and tracks. Overlap and end-of-playlist rules are the same as Cmd+V (Q152a).

### 19. Messages

All short messages from this project appear in the notice banner: info style for refusals (paste or repeat past the end, join needs one track, join needs one kind, join clips not on the same grid), error style for failures (deep copy or join failed) (F37).

### 4. Undo and redo

#### 4.1 What is undoable

- Adding, moving and deleting clips; clip settings (gain, pan, pitch, attack, decay); exploding a pattern clip; mixer changes (volume, pan, mute, solo, track names, reset); Sequencer edits (rows, patterns, steps, bars, names, folders); tempo; and every new operation in this spec (Q10, Q11).
- Not undoable: clips placed by a finished recording; the metronome, loop, snap and count-in toggles and the snap value; track arming (Q11b). File operations in the Recordings and Clips pages (F9).
- Never undo steps: selection changes, tool changes, zoom, scroll, playhead moves, play and stop, tab and pane layout (Q17).

#### 4.2 Steps

One gesture is one step (Q14):

| Gesture | Steps |
|---|---|
| One edge drag, trim or stretch, however many clips it changes | 1 |
| One cut, however many clips it cuts | 1 |
| One Paste, Cmd+X or Repeat | 1 |
| One Deep copy, however many clips (G2) | 1 |
| One Join | 1 |
| One clip or selection move | 1 |
| Deleting a selection | 1 |
| Dropping one clip in | 1 |
| One knob or fader drag, press to release | 1 |
| One mouse stroke painting Sequencer steps | 1 |
| One visit to the tempo field, from focus to leaving it | 1 |
| Each mute, solo, rename, add or remove | 1 |

#### 4.3 Rules

- Cmd+Z undoes; Shift+Cmd+Z redoes (Q13). They act on the project whenever you are not typing in a text field, whichever view is focused. In a text field they undo typing (Q19).
- The history holds 100 steps (Q15).
- It is cleared by New Project and Open, survives Save, and is not written to the project file (Q16).
- After you undo and then make a new edit, the undone steps can no longer be redone (F11).
- After an undo or redo, the clips it changed or restored become the Playlist selection (Q18).
- Undo and redo do nothing while a recording is in progress or while a mouse drag is in progress (F8).
- A finished recording leaves the history intact; its clips stay as if they had always been there (F7).
- Undo never deletes a file on disk (Q119, F9).
- The history lives in the engine. The frontend asks the engine to undo or redo and redraws from what the engine returns (Q12).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### K. Deep copy (M11)

- [ ] K1 [M] "Deep copy" on a trimmed, pitched clip: the clip stays where it was, its label becomes `<name>_copy_1.wav`, the Sample tab shows Pitch 0 and no fades, and a file of that name exists in `<project>/clips/`.
- [ ] K2 [E] The copy rendered back has the same number of frames as the original clip rendered, and every sample is within one 16-bit step.
- [ ] K3 [E] With gain +12 dB on a loud file, the copy equals the original clipped to full scale, within one 16-bit step.
- [ ] K4 [E] The file is WAV, 32-bit float, stereo, at the engine's rate.
- [ ] K5 [E] A second deep copy of the same source is named `_copy_2`; an existing `Kick_Copy_1.wav` is not overwritten by `kick_copy_1.wav`.
- [ ] K6 [M] In a never-saved project, Deep copy opens the Save dialog; after saving, the copy is made; cancelling leaves the clip unchanged and writes nothing.
- [ ] K7 [M] Undo puts the clip back on its original file with its trim and pitch; the new file is still in `clips/`. Redo uses that file and no `_copy_2` appears.
- [ ] K8 [M] Deep copy on three selected clips writes three files and is one undo step.
- [ ] K9 [E] If the second of three files cannot be written, no clip changes and no new file remains.
- [ ] K10 [M] A failure shows its reason in the notice banner.
- [ ] K11 [M] Save As to a new folder copies `clips/` there, and the new project's clips play from the copies.
- [ ] K12 [M] Deep copy on a pattern clip turns it into a sample clip of the same length named `<pattern>_copy_1.wav`; undo turns it back.
- [ ] K13 [E] A one-minute clip is written in under 2 seconds.
- [ ] K14 [M] The source file the clip used before is still on disk, unmoved.

### J. Context menus (M10, M11, M13)

- [ ] J1 [M] A sample clip's menu reads: Cut, Copy, Paste, Repeat; divider; Split at playhead, Join, Deep copy; divider; Delete. Paste is absent with an empty clipboard, Join with fewer than two clips selected, Split when the playhead does not cross the clip.
- [ ] J2 [M] A pattern clip's menu has "Explode into tracks" and "Open in Sequencer" above those.
- [ ] J3 [M] Right-clicking one of three selected clips and choosing Delete removes all three.
- [ ] J4 [M] Right-clicking an unselected clip leaves the selection as it was, and Delete removes only that clip.
- [ ] J5 [M] Right-clicking empty space at bar 5 on track 2 with clips on the clipboard opens a menu with Paste; choosing it puts the earliest clip at bar 5 with the topmost copied clip on track 2. With an empty clipboard no menu opens.
- [ ] J6 [M] The menu looks as it did, apart from the new items and divider lines.

### Things that must never happen (relevant here)

- [ ] X1 [E] Editing one clip (trim, stretch, cut, settings, deep copy) changes what another clip plays.
- [ ] X2 [E] A deep copy or join writes a file anywhere outside the project's folder.
- [ ] X3 [E] A deep copy or join overwrites an existing file.
- [ ] X4 [E] Undo, redo, deep copy or join deletes or moves an audio file.
- [ ] X8 [E] A failed deep copy or join leaves a clip pointing at a missing or partly written file.
