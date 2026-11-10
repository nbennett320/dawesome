# Handoff brief — M10 — Clipboard, Repeat, Select All, context menus

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

## Spec excerpts

### 13. Clipboard and Repeat

#### 13.1 Copy and cut

- Cmd+C copies every selected clip, sample and pattern, with its track, its position relative to the others, its trim and its settings (Q123, Q26).
- Cmd+X does the same and removes the clips, leaving a gap; nothing else moves (Q131).
- With nothing selected, Cmd+C and Cmd+X do nothing (Q123).
- The clip clipboard is private to Dawesome. It does not change the system clipboard. In a text field, Cmd+C, Cmd+V and Cmd+X act on the text (Q134).
- New Project and Open empty the clipboard (Q133).

#### 13.2 Paste (Cmd+V)

- The earliest copied clip starts at the playhead and the rest keep their spacing (Q124).
- The position is snapped to the grid when Snap is on and exact when Snap is off, whether playing, stopped or paused. While playing, the playhead's position at the moment of the key press is used (Q125, G1).
- Clips go on the tracks they were copied from (Q126).
- A paste may overlap existing clips (Q127).
- If any pasted clip would start past the end of the playlist, nothing is pasted and the notice banner says why (Q128, F37).
- Pressing Cmd+V again pastes in the same place (Q129).
- The pasted clips become the selection (Q130).
- A pasted sample clip plays the same file as its original and has its own independent trim and settings. A pasted pattern clip is linked to the same pattern. No file is written (Q132).
- A pattern clip whose pattern no longer exists is skipped; the rest is pasted (H2).
- With an empty clipboard, Cmd+V does nothing (F38).

#### 13.3 Repeat (Cmd+R)

- Repeat duplicates the selection so that the duplicate starts exactly where the selection's span ends. The span runs from the earliest start to the latest end. There is no rounding (Q137).
- Duplicates go on the same tracks as their originals and keep the gaps between them (Q139).
- The new duplicates become the selection (Q138).
- Past the end of the playlist, nothing is repeated and the notice banner says why (Q139, Q128).
- Duplicates share audio like pasted clips. Repeat does not change the clipboard. With nothing selected it does nothing (Q139).
- Cmd+R never reloads the app window, in development or release builds, whichever view is focused (Q140).

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

### 7. Selection and lasso

- Holding Shift is the selection modifier, as now (Q28).
- Shift+drag starts a lasso anywhere, including over a clip, once the mouse has moved 10 px (Q39, Q40). Shift+click without moving still toggles the clip (Q39).
- A lasso started over a clip selects that clip, because the box touches it (Q41).
- A lasso always adds to the selection (Q42). Its hit rule is unchanged: the box touches the clip's lane-height box (Q43).
- With Shift held, a drag that starts in a clip's edge zone stretches (section 9) instead of starting a lasso; a Shift+click with no drag toggles the clip even in an edge zone (Q81).
- Cmd+A selects every clip, when Playlist-focused (Q136, Q162).

### 5. Edit menu

- The menu bar gains an Edit menu: Undo, Redo; divider; Cut, Copy, Paste, Repeat, Select All; divider; Join (Q135, F10).
- Undo and Redo keep plain labels (F10).
- The items act on clips when the Playlist is focused and on text while typing in a text field (Q135, Q134).

### 18. Shortcuts

Approved in Q162. On macOS only Cmd works for the Cmd shortcuts; on other platforms only Ctrl (Q160). Letters are matched by the character the key types (Q161).

| Keys | Action | Scope | Status |
|---|---|---|---|
| Space | Play / pause | Everywhere, not typing | New |
| S | Select tool | Playlist-focused, no modifier | New |
| C | Cut tool | Playlist-focused, no modifier | New |
| Cmd+C / Cmd+X / Cmd+V | Copy / cut / paste clips | Playlist-focused (text in text fields) | New |
| Cmd+R | Repeat | Playlist-focused; reload blocked everywhere | New |
| Cmd+A | Select all clips | Playlist-focused | New |
| Shift+J | Join | Playlist-focused | New |
| Cmd+Z / Shift+Cmd+Z | Undo / redo | Any view, not typing | New |
| Option held | Bypass snap during trim, stretch and cut | Playlist drags | New |
| Shift+drag a clip edge | Stretch | Select tool | New |
| Shift+drag over a clip | Lasso | Playlist | Changed |
| Escape | Clear the selection; cancel a drag | As today | Unchanged |
| Shift+click | Toggle a clip in the selection | Playlist | Unchanged |
| Shift+E | Explode selected pattern clips | Playlist-focused | Unchanged |
| Delete / Backspace | Delete selected clips | Playlist-focused | Unchanged |
| Cmd+N, Cmd+O, Cmd+S, Shift+Cmd+S | New, Open, Save, Save As | Menu | Unchanged |
| Shift+Cmd+M, Shift+Cmd+E | Open Mixer, open Sequencer | Menu | Unchanged |
| Cmd+wheel, pinch | Zoom | Playlist canvas | Unchanged |

### 19. Messages

All short messages from this project appear in the notice banner: info style for refusals (paste or repeat past the end, join needs one track, join needs one kind, join clips not on the same grid), error style for failures (deep copy or join failed) (F37).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### I. Clipboard and Repeat (M10)

- [ ] I1 [M] Select two clips on tracks 1 and 3, Cmd+C, click the ruler at bar 9, Cmd+V: copies appear on tracks 1 and 3, the earlier one starting at bar 9, the other at the same distance after it, and they are selected.
- [ ] I2 [M] Cmd+V again puts a second set in the same place.
- [ ] I3 [M] Cmd+X removes the clips and nothing else moves.
- [ ] I4 [E] Changing a pasted clip's trim or settings leaves the original's rendered output unchanged, and no file is written by a paste.
- [ ] I5 [U] A paste in which any clip would start past bar 64 pastes nothing; the notice banner explains.
- [ ] I6 [M] While playing with Snap on, Cmd+V lands on the grid line nearest the playhead at the moment of the press.
- [ ] I7 [M] Copy a clip, open another project, Cmd+V: nothing is pasted.
- [ ] I8 [M] Copy text in another app, copy clips in Dawesome, paste in the other app: the text is pasted.
- [ ] I9 [M] In a rename field, Cmd+C, Cmd+X and Cmd+V act on the text and no clip changes.
- [ ] I10 [U] Repeat on a selection spanning beats 2 to 5.7 puts the duplicate's earliest clip at beat 5.7 exactly, on the same tracks, with the same gaps.
- [ ] I11 [M] Pressing Cmd+R three times lays three copies end to end, the last one selected.
- [ ] I12 [M] Cmd+R never reloads the window, in `pnpm dev` and in a release build, with any view focused.
- [ ] I13 [M] Paste, Cmd+X and Repeat are each one undo step.
- [ ] I14 [M] A pattern clip can be copied and pasted; the paste plays the same pattern.

### J. Context menus (M10, M11, M13)

- [ ] J1 [M] A sample clip's menu reads: Cut, Copy, Paste, Repeat; divider; Split at playhead, Join, Deep copy; divider; Delete. Paste is absent with an empty clipboard, Join with fewer than two clips selected, Split when the playhead does not cross the clip.
- [ ] J2 [M] A pattern clip's menu has "Explode into tracks" and "Open in Sequencer" above those.
- [ ] J3 [M] Right-clicking one of three selected clips and choosing Delete removes all three.
- [ ] J4 [M] Right-clicking an unselected clip leaves the selection as it was, and Delete removes only that clip.
- [ ] J5 [M] Right-clicking empty space at bar 5 on track 2 with clips on the clipboard opens a menu with Paste; choosing it puts the earliest clip at bar 5 with the topmost copied clip on track 2. With an empty clipboard no menu opens.
- [ ] J6 [M] The menu looks as it did, apart from the new items and divider lines.

### E. Lasso and selection (M6, M8, M10)

- [ ] E1 [M] Shift+drag starting on a clip draws a selection box once the mouse has moved 10 px, and that clip is selected.
- [ ] E2 [M] Shift+click on a clip without moving toggles it, as before.
- [ ] E3 [M] A plain drag on empty space still pans; a plain drag on a clip still moves it; a plain click on a clip still selects nothing.
- [ ] E4 [M] A second lasso adds to the first selection.
- [ ] E5 [M] Cmd+A with the Playlist focused selects every clip. In a text field it selects the text.

### Things that must never happen (relevant here)

- [ ] X6 [M] C, S, Shift+J, Cmd+C, Cmd+X, Cmd+V, Cmd+R or Cmd+A changes the project while typing in a text field.
- [ ] X7 [M] Cmd+R reloads the app window.
- [ ] X12 [E] A paste, repeat or multi-clip cut applies to only some of its clips after a failure (each is all or nothing).
