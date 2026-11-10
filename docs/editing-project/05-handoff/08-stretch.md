# Handoff brief — M8 — Stretch

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

**Goal:** Shift+dragging an edge stretches by writing Pitch (§9), and a Shift-press in an edge zone is told apart from a lasso or a toggle (§7).

**Work:** the edge drag from M7 gains a stretch mode fixed at the press. The target length gives a pitch value, limited to ±24 semitones. With a left-edge stretch the start moves and the end stays. Multi-selection applies the same ratio. One engine call on release sets pitch (and start, for a left-edge stretch) for every affected clip as one undo step.

**Files:** `src/views/playlist/sketch/index.ts`, `edges.ts`, `edges.spec.ts`, `src/state/slices/playlistSlice.ts`, `engine/src/event_dispatch.rs` (a batched "set settings and placement" command), `engine/src/history.rs`

**Risks**

- Pitch is stored as a 32-bit float and the engine truncates the pitched rate to whole Hz, so a clip stretched "to bar 3" can end a fraction of a millisecond away. The acceptance item states the tolerance (one output frame).
- A stretched clip's fades are in milliseconds and are clamped when it gets shorter (existing rule).

**Tests:** `edges.spec.ts`: length-to-pitch and back; the ±24 limit stops the edge; left-edge stretch keeps the end; same ratio across a selection with one clip at its limit; pattern clips are skipped. Engine: a clip stretched to twice its length renders twice as many frames (within one frame) and reports pitch −12.

- [ ] Approved

## Spec excerpts

### 9. Stretch

- Shift+dragging an edge stretches the clip to the cursor (Q79, Q81).
- Stretch is tape-style: it changes speed and pitch together (Q72). It works by writing the clip's Pitch setting, which the Sample tab shows (Q77).
- The opposite edge stays where it is (Q79).
- Stretch applies to what the clip currently plays; it does not reveal trimmed-away audio (Q80).
- The limit is Pitch's ±24 semitones: a quarter to four times the unpitched length. The edge stops there (Q77).
- It snaps like a trim: grid, playhead and Option bypass (Q83, F32).
- The clip and its waveform squeeze or expand live; the engine is told on release (Q84).
- A stretched clip shows the amber "modified" dot, because its Pitch is no longer 0. There is no other indicator (Q85).
- A stretch is removed by setting Pitch back to 0 in the Sample tab, or by undo. There is no menu item for it (Q86).
- A stretched clip keeps its length in seconds when the tempo changes (Q88).
- When the grabbed clip is part of a selection, every selected sample clip is stretched by the same ratio, each keeping its own opposite edge fixed and stopping at its own limit. Pattern clips in the selection are left as they are (Q89, F34).
- Stretch does not apply to pattern clips (Q26). A Shift+drag on a pattern clip's edge does nothing (Q27).
- Escape, release outside the canvas, minimum length, overlap and "heard from the next start" follow section 8 (Q69, Q70, Q55, Q59, Q63).

### 7. Selection and lasso

- Holding Shift is the selection modifier, as now (Q28).
- Shift+drag starts a lasso anywhere, including over a clip, once the mouse has moved 10 px (Q39, Q40). Shift+click without moving still toggles the clip (Q39).
- A lasso started over a clip selects that clip, because the box touches it (Q41).
- A lasso always adds to the selection (Q42). Its hit rule is unchanged: the box touches the clip's lane-height box (Q43).
- With Shift held, a drag that starts in a clip's edge zone stretches (section 9) instead of starting a lasso; a Shift+click with no drag toggles the clip even in an edge zone (Q81).
- Cmd+A selects every clip, when Playlist-focused (Q136, Q162).

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

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### G. Stretch (M8)

- [ ] G1 [E] Shift+dragging the right edge of a 2-bar clip that starts at bar 1 out to the start of bar 5 (twice its length) sets its Pitch to −12.00 and it renders twice as many frames, within one frame.
- [ ] G2 [M] Shift+dragging the left edge moves the start to the cursor and leaves the end where it was.
- [ ] G3 [M] The edge stops when Pitch reaches ±24 semitones.
- [ ] G4 [M] The Sample tab's Pitch shows the new value, the clip has the amber dot, and setting Pitch to 0 restores the unstretched length.
- [ ] G5 [M] Stretching a trimmed clip squeezes only the trimmed content; no hidden audio is revealed.
- [ ] G6 [M] Pressing or releasing Shift mid-drag does not change what the drag does.
- [ ] G7 [M] With several clips selected, all sample clips change by the same ratio; pattern clips do not change.
- [ ] G8 [M] Shift+dragging a pattern clip's edge does nothing.
- [ ] G9 [M] A Shift+drag starting in an edge zone stretches; starting elsewhere on the clip it draws a lasso; a Shift+click in an edge zone toggles the clip.

### Things that must never happen (relevant here)

- [ ] X1 [E] Editing one clip (trim, stretch, cut, settings, deep copy) changes what another clip plays.
- [ ] X13 [M] A trim, cut or stretch moves a clip the user did not select or grab.
