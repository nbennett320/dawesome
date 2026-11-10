# Handoff brief — M12 — Clips sidebar page

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

**Goal:** a Clips page that behaves like the Recordings page, pointed at `clips/` (§12).

**Work:** generalise the recordings commands and the Recordings browser so both pages share one implementation that takes the folder as a parameter; add the rail key and its icon (shown to you before it ships).

**Files:** `engine/src/audio_commands.rs`, `engine/src/project_files.rs`, `engine/src/main.rs`, `src/components/toolbars/recordings-browser/RecordingsBrowser.tsx` (shared browser extracted), new `src/components/toolbars/clips-browser/ClipsBrowser.tsx`, `src/components/sidebar/Sidebar.tsx`, `src/types/ui.ts`, `src/types/recordings.ts`, `src/state/slices/windowSlice.ts`, new `src/components/icons/ClipsIcon.tsx`

**Risks:** the recordings commands also drive take naming and the unsaved-session folder, which do not apply to `clips/`; the shared code must keep those paths recordings-only. This is a refactor of working code, so the existing recordings tests are the safety net.

**Tests:** engine: list, rename (clips using the file follow), move, folder, delete (refused while in use) against a temporary `clips/`; all existing recordings tests still pass. Frontend: the existing `recordings.spec.ts` helpers run for both folders.

- [ ] Approved

## Spec excerpts

### 12. Clips sidebar page

- The sidebar gains a "Clips" page with a key on the rail below Samples, Patterns and Recordings (Q116, F28). Its icon is drawn in the existing line style and approved by you (F28).
- It behaves exactly like the Recordings page does today, pointed at `clips/`: search, preview on press, dragging into the Playlist or Sequencer, the "in use" marker, renaming with clips that use the file following along, deleting to the Trash (refused while the project uses the file), and folders (Q116a, F27).
- Files made by Join appear there too (F27).
- Its file operations are not undo steps (F9).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### L. Clips page (M12)

- [ ] L1 [M] The sidebar rail has a Clips key below Recordings; its page lists the files in `clips/`.
- [ ] L2 [M] Pressing a file plays it; dragging it onto a track places a clip.
- [ ] L3 [M] Renaming a file that a clip uses renames it on disk and the clip keeps playing, with the new label.
- [ ] L4 [M] Deleting a file in use is refused with the Recordings page's message; deleting an unused file moves it to the Trash.
- [ ] L5 [M] Folders can be created and files moved into them, and clips using those files keep playing.
- [ ] L6 [M] The Recordings page behaves exactly as before.
