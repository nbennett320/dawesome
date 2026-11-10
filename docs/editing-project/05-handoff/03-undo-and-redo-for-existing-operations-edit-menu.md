# Handoff brief — M3 — Undo and redo for existing operations, Edit menu

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

## Spec excerpts

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

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### B. Undo and redo (M3, extended by each later milestone)

- [ ] B1 [E] For every undoable command: apply, undo → the project equals the project before; redo → it equals the project after.
- [ ] B2 [M] Move a clip, Cmd+Z: it returns and is selected. Shift+Cmd+Z: it moves again.
- [ ] B3 [M] Delete five selected clips, Cmd+Z once: all five return.
- [ ] B4 [M] Drag a mixer fader from −6 to 0 in one motion, Cmd+Z once: it is back at −6.
- [ ] B5 [M] Paint eight Sequencer steps in one stroke, Cmd+Z once: all eight clear.
- [ ] B6 [M] Type a new tempo and leave the field, Cmd+Z once: the old tempo returns.
- [ ] B7 [E] After 150 edits, exactly 100 can be undone.
- [ ] B8 [E] Undo three steps, make one edit: redo does nothing.
- [ ] B9 [M] Cmd+Z works with the Mixer or Sequencer focused.
- [ ] B10 [M] Cmd+Z while typing in a rename field undoes typing and changes nothing in the project.
- [ ] B11 [E] The history is empty after New Project and after Open, and unchanged by Save.
- [ ] B12 [E] Undo is refused while a recording is in progress.
- [ ] B13 [M] Toggling metronome, loop or snap, or arming a track, adds no undo step.
- [ ] B14 [M] The Edit menu shows Undo, Redo, Cut, Copy, Paste, Repeat, Select All, Join, with plain Undo and Redo labels.

### Things that must never happen (relevant here)

- [ ] X4 [E] Undo, redo, deep copy or join deletes or moves an audio file.
- [ ] X6 [M] C, S, Shift+J, Cmd+C, Cmd+X, Cmd+V, Cmd+R or Cmd+A changes the project while typing in a text field.
