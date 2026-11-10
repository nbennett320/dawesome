# Handoff brief — M6 — Lasso over a clip

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

**Goal:** Shift+drag starts a lasso anywhere after 10 px; Shift+click still toggles (§7, without the edge-zone exception, which arrives with M8).

**Work:** a Shift-press records a pending gesture instead of acting at once. Movement past the threshold starts the lasso from the press point; release without movement toggles the clip under the press, or does nothing on empty space.

**Files:** `src/views/playlist/sketch/index.ts`, `src/views/playlist/sketch/selection.ts` (press-resolution function), `selection.spec.ts`

**Risks:** the lasso currently exists from the moment of the press; drawing and auto-scroll must wait for the threshold.

**Tests:** press-resolution function: press on clip with no movement toggles; movement of 9 px does not start a lasso, 11 px does; a lasso started on a clip includes it; the result always adds to the existing selection.

- [ ] Approved

## Spec excerpts

### 7. Selection and lasso

- Holding Shift is the selection modifier, as now (Q28).
- Shift+drag starts a lasso anywhere, including over a clip, once the mouse has moved 10 px (Q39, Q40). Shift+click without moving still toggles the clip (Q39).
- A lasso started over a clip selects that clip, because the box touches it (Q41).
- A lasso always adds to the selection (Q42). Its hit rule is unchanged: the box touches the clip's lane-height box (Q43).
- With Shift held, a drag that starts in a clip's edge zone stretches (section 9) instead of starting a lasso; a Shift+click with no drag toggles the clip even in an edge zone (Q81).
- Cmd+A selects every clip, when Playlist-focused (Q136, Q162).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### E. Lasso and selection (M6, M8, M10)

- [ ] E1 [M] Shift+drag starting on a clip draws a selection box once the mouse has moved 10 px, and that clip is selected.
- [ ] E2 [M] Shift+click on a clip without moving toggles it, as before.
- [ ] E3 [M] A plain drag on empty space still pans; a plain drag on a clip still moves it; a plain click on a clip still selects nothing.
- [ ] E4 [M] A second lasso adds to the first selection.
- [ ] E5 [M] Cmd+A with the Playlist focused selects every clip. In a text field it selects the text.
