# Handoff brief — M4 — Clip windows in the engine

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

## Spec excerpts

### 3. Data model

- Two clips are "instances of the same source" when they point at the same audio file. There is no other link between them (Q21).
- A trimmed sample clip is its file plus two per-clip values: where in the file it starts and how much of the file it plays. The file is never rewritten (Q22).
- A trimmed pattern clip is a window on its pattern in the same way (Q26, F13).
- No edit to one clip changes any other clip, including clips of the same file and the other half of a cut (Q23).
- Clips have no name or colour property. A sample clip's label is its file name (Q96).

#### 3.1 Project file

- A project is written as format version 3 only if some clip is trimmed. Otherwise it is written as it is today (version 1 or 2). Older builds refuse version 3 files with the existing "saved by a newer version" message (Q24).
- Existing version 1 and 2 projects open with every clip untrimmed, and nothing in the file is rewritten until you save (Q25).

### 8. Trim

#### 8.1 Grabbing an edge

- Each edge has a grab zone 6 screen pixels wide, inside the clip, at every zoom level (Q47), running the clip's full height (Q49).
- On a narrow clip each zone shrinks to at most a third of the clip's width, so the middle third always moves the clip (Q48).
- Over a zone the cursor is `ew-resize` (Q50) and a 2 px bar in the sky accent colour is drawn on that edge (Q51).
- Pressing an edge zone affects the selection exactly as pressing the clip's middle does: pressing an unselected clip clears the selection and does not select it (Q65, Q44).
- Edge drags exist only in the Select tool (Q37).

#### 8.2 Dragging

- A plain drag of an edge trims: it moves the clip's start or end (Q22).
- The audio stays where it is on the timeline. Dragging the left edge right hides the start; dragging it back reveals it (Q57).
- The clip resizes live under the mouse, revealing or hiding waveform. The engine is told once, on release (Q52).
- Nothing is heard while dragging (Q62).
- Whether Shift was held is decided at the press and does not change during the drag (Q82).

#### 8.3 Limits

- The left edge stops at the source's first sample or at beat 0, whichever comes first (Q57). An edge stops at the end of the source audio (Q56).
- The right edge may go past the playlist's last beat, as far as the source allows (Q58).
- A sample clip cannot be shorter than 10 ms of played audio (Q55). A pattern clip cannot be shorter than one sixteenth-note step (F15).
- A trim may make a clip overlap its neighbour; both play (Q59).

#### 8.4 Snapping

- With Snap on, the dragged edge's timeline position snaps to the nearest snap step (Q53).
- The playhead is also a snap point: the edge goes to the playhead when the cursor is nearer to it than to either neighbouring grid line. This applies only with Snap on, Option not held, and playback stopped or paused (Q91b, F32).
- Holding Option bypasses snap for trim, stretch and cut. Existing clip moves and drops are unaffected (Q54).

#### 8.5 Fades

- Fades follow the trimmed edges: the fade-in starts at the clip's current left edge and the fade-out ends at its current right edge (Q60).
- If a trim makes the clip shorter than a fade, the stored fade is shortened to fit, as it is today. Dragging the edge back out does not restore it; undo does (Q61).

#### 8.6 Several clips

- When the grabbed clip is part of a selection, every selected clip moves the same edge by the same amount of time (Q64, F33).
- Only the grabbed clip's edge snaps; the others move by that same amount (F33).
- Each clip stops at its own source boundary and minimum length (Q64, F33). Pattern clips in the selection trim along with the rest (F33).

#### 8.7 Ending the drag

- Releasing applies the trim, including when the mouse is outside the canvas (Q70).
- Escape during the drag puts the clip back and sends nothing to the engine (Q69).
- The view does not auto-scroll sideways (Q71).
- An edit that lands while the clip is sounding is heard from the clip's next start (Q63).

#### 8.8 Appearance

- An edge that hides audio or steps beyond it shows a small solid triangle in the label band at that edge, pointing inward, in the clip's border colour (Q66, F29).
- A trim does not show the amber "modified" dot (Q66).
- In the Sample tab, a trimmed clip shows the whole file with the played part highlighted and the rest dimmed. The fade envelope and its handles sit inside the highlight. The Length readout shows the played length. The highlight cannot be dragged (Q67, F30).
- The Sample tab's Reset returns the five settings only; trim is left alone (Q87).
- There are no numeric trim fields (Q68).

### 10. Cut

#### 10.1 Ways to cut

- In the Cut tool, clicking a clip splits it at the click position (Q90).
- "Split at playhead" in a clip's context menu splits at the playhead (Q90). While playing it uses the playhead's position at the moment you choose it, unsnapped. The item is left out when the playhead does not cross the clip (F31).
- There is no cut shortcut (Q90).

#### 10.2 Cut tool behaviour

- The cut lands at the mouse position, snapped to the grid when Snap is on (Q91).
- The playhead is also a snap point, under the same rule as section 8.4: Snap on, Option not held, playback stopped or paused (Q91, Q91a, Q91c).
- Holding Option bypasses snap (Q54).
- A vertical dashed line in the accent colour on the hovered clip shows where the cut will land (Q92).
- The cursor is a custom razor blade, drawn as an SVG in the existing icon style and approved by you, with the crosshair as its fallback (Q93, Q93a).
- The cut happens on release, if the mouse has not moved more than 10 px since the press. Moving away first cancels (Q100).
- If the position is closer to an edge than the minimum clip length, no cut is made and no line is shown (Q99).
- The clip under the cursor is always cut. If it is part of a selection, every other selected clip crossing that position is cut too (Q98).

#### 10.3 Result

- A cut produces two clips that are windows on the same file or pattern (Q22, F19).
- The left half is the original clip: it stays selected and inspected if it was. The right half is a new, unselected clip (Q97).
- Both halves keep the original's track, gain, pan and pitch (Q96).
- The left half keeps the fade-in and has no fade-out. The right half keeps the fade-out and has no fade-in (Q94).
- A fade longer than the half it lands on is shortened to that half (Q95).
- Played back, the two halves produce exactly the same output frames as the uncut clip, at any pitch and any file sample rate, except where a fade was shortened under the rule above (F35).

### 15. Pattern clips: summary

| Feature | Applies | Source |
|---|---|---|
| Copy, cut, paste, Repeat | Yes | Q26 |
| Trim | Yes | Q26 |
| Cut tool, Split at playhead | Yes | Q26 |
| Join | Yes | Q26 |
| Deep copy | Yes | Q26 |
| Stretch | No | Q26 |

- A trimmed pattern clip plays the hits that start inside its window; each rings out naturally, as hits do today (F13).
- Its right edge stops at the pattern's end (F14).
- It shows only the steps inside its window, at the same step width, with the same edge triangles as sample clips (F18).
- If the pattern's length is changed later, the clip keeps its window, shortened to the pattern's end where needed. It remembers its original window, so lengthening the pattern brings it back. A window entirely past the pattern's end leaves a one-step clip that plays nothing. Untrimmed clips keep following the pattern (F16, F17).
- Where a feature does not apply to a clip, nothing happens to that clip, its menu item is left out, and a mixed selection applies the action only to the clips it fits (Q27).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### C. Clip windows and project file (M4)

- [ ] C1 [E] A whole clip sounds as it did before this project: the new reader's output equals the old path's within one 16-bit step.
- [ ] C2 [E] A project with no trimmed clip is still written as version 1 or 2, and an older-format fixture loads with every clip whole.
- [ ] C3 [E] A project with a trimmed clip is written as version 3 and reloads with the same window.
- [ ] C4 [E] Changing one clip's window leaves the rendered output of another clip of the same file unchanged, sample for sample.

### Things that must never happen (relevant here)

- [ ] X1 [E] Editing one clip (trim, stretch, cut, settings, deep copy) changes what another clip plays.
- [ ] X9 [E] An older-format project is rewritten or changed merely by opening it.
- [ ] X10 [E] A project with a trimmed clip is written with a version lower than 3 (which would let an older build open it and play the clip untrimmed).
- [ ] X11 [E] A whole, unedited clip sounds different from how it sounded before this project, beyond one 16-bit step.
