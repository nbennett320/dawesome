# Handoff brief — M7 — Trim

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

## Spec excerpts

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

### F. Trim (M7)

- [ ] F1 [M] Within 6 px inside either edge of a clip the cursor becomes the horizontal-resize cursor and a 2 px sky bar appears on that edge.
- [ ] F2 [M] On a clip 12 px wide, the middle third still moves it.
- [ ] F3 [E] Dragging the right edge of a 4-bar clip to bar 2 with Snap on leaves a 2-bar clip whose last rendered frame is the same source sample that played at bar 2 before.
- [ ] F4 [E] Dragging the left edge of that clip from bar 1 to bar 2 leaves a clip starting at bar 2 whose first rendered frame is the source sample that played at bar 2 before.
- [ ] F5 [M] While dragging, the clip resizes live and the waveform under the mouse does not slide.
- [ ] F6 [M] The right edge cannot be dragged past the end of the audio, and the left edge cannot be dragged before its start or before bar 1.
- [ ] F7 [U] A clip cannot be made shorter than 10 ms; a pattern clip not shorter than one sixteenth-note step.
- [ ] F8 [M] With Option held the edge follows the mouse exactly; released, it snaps again.
- [ ] F9 [M] Stopped, with the playhead at beat 2.3 and the mouse at beat 2.35, the edge lands on the playhead. With the mouse at beat 2.9 it lands on beat 3. While playing, the playhead is not a snap point.
- [ ] F10 [E] A clip with a 100 ms fade-in, trimmed 1 second in from the left, fades in over the first 100 ms after its new start.
- [ ] F11 [M] With three clips selected, dragging one's right edge left by one beat shortens all three by one beat; one that reaches its minimum stops while the others continue.
- [ ] F12 [M] Escape during the drag restores the clip. Releasing outside the canvas applies the trim.
- [ ] F13 [M] A trimmed edge shows a small inward triangle in the label band; an untrimmed edge does not. A trimmed clip does not gain the amber dot.
- [ ] F14 [M] The Sample tab shows the whole file with the played part highlighted, the rest dimmed, and the fade handles inside the highlight. Reset leaves the trim alone.
- [ ] F15 [M] A trimmed pattern clip shows only the steps in its window and plays only the hits that start inside it.
- [ ] F16 [M] Trimming is one undo step, including a multi-clip trim.
- [ ] F17 [M] Trimming a clip while it is sounding changes nothing audible until the clip next starts.
- [ ] F18 [M] In the Cut tool, edges cannot be dragged.

### Things that must never happen (relevant here)

- [ ] X1 [E] Editing one clip (trim, stretch, cut, settings, deep copy) changes what another clip plays.
- [ ] X13 [M] A trim, cut or stretch moves a clip the user did not select or grab.
