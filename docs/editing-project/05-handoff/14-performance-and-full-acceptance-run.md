# Handoff brief — M14 — Performance and full acceptance run

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

**Goal:** the performance bar holds and every item in `04-acceptance.md` is checked (§20).

**Work:** generate a 500-clip test project; measure frame rate during an edge drag, a cut hover and a lasso with the canvas's built-in readout; time each edit command; fix what misses the bar. Run the whole acceptance checklist, with a verifier subagent checking code against spec for every section.

**Files:** whatever the measurements point at; a script under `docs/editing-project/` that writes the test project.

**Risks:** the canvas rebuilds every clip object when the clip list changes; a 500-clip paste may need the rebuild to be incremental.

**Tests:** engine timing tests for batched add, delete, set-window and split on 500 clips (each under 50 ms).

- [ ] Approved

## Spec excerpts

### 20. Quality bars

- **Performance:** with 500 clips in the project, edge drags, cut previews and lassos stay at 50 frames per second or better on your Mac, and every edit command returns within 50 ms (Q163).
- **Look and feel:** no existing style changes. New canvas marks use colours already in `theme.ts`. New controls are built from existing four-track and sidebar components. Cursors are standard, except the razor blade (Q165).
- **Platform:** verified on macOS only (Q4).

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

### N. Shortcuts (M5–M13)

- [ ] N1 [U] On macOS, Ctrl+C, Ctrl+V, Ctrl+X, Ctrl+R, Ctrl+A and Ctrl+Z do nothing to clips.
- [ ] N2 [M] Every row of the table in spec §18 behaves as listed.
- [ ] N3 [M] Shift+E, Delete, Cmd+N, Cmd+O, Cmd+S, Shift+Cmd+S, Shift+Cmd+M and Shift+Cmd+E behave as before.

### O. Performance and look (M14)

- [ ] O1 [M] With the 500-clip test project, the frame-rate readout stays at 50 or above during an edge drag, a cut hover and a lasso.
- [ ] O2 [E] Batched add, delete, set-window and split on a 500-clip project each return within 50 ms.
- [ ] O3 [M] No existing control, menu or colour looks different from before, apart from the snap controls' new place.
