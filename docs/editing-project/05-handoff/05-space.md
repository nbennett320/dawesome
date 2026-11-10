# Handoff brief — M5 — Space

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

**Goal:** Space is the Play/Pause key everywhere except while typing (§17).

**Work:** one window-level key handler that calls the same action as the topbar Play/Pause key, ignores key repeat, ignores typing targets, and prevents the default so a focused button is not also "clicked". Remove Space from the per-control handlers that use it (they keep Enter).

**Files:** `src/util/keys.ts`, `src/App.tsx` or a small new hook, `src/views/sequencer/SequencerRow.tsx`, `src/views/transport/TransportDeck.tsx`, `src/components/toolbars/sample-browser/SampleItem.tsx`, `src/components/toolbars/recordings-browser/RecordingsBrowser.tsx`

**Risks:** react-tabs and other focusable controls may have their own Space handling; the handler listens in the capture phase so it wins.

**Tests:** key predicate unit tests (Space with and without modifiers, repeat, each typing target type); a thunk test that Space dispatches the same action as the Play/Pause key in Playlist and Sequencer focus.

- [ ] Approved

## Spec excerpts

### 17. Space

- Space does what the topbar Play/Pause key does, including looping the selected pattern when the Sequencer is the focused view (Q154).
- It works everywhere, whichever view is focused, except while typing (Q155).
- While typing in a text field, the tempo field, a rename field, search, a drop-down list or editable text, Space never starts or stops playback (Q156).
- Outside those, Space always means transport. Focused step cells, sidebar rows, wind keys and buttons no longer react to Space; they keep Enter (Q157).
- Holding Space toggles once (Q158).
- During a recording, Space stops playback and finishes the take, as the Play/Pause key does (Q159).

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

### D. Space (M5)

- [ ] D1 [M] Space starts playback; Space again pauses, and the playhead stays where it stopped.
- [ ] D2 [M] With the Sequencer focused and a pattern selected, Space loops that pattern, as the Play/Pause key does.
- [ ] D3 [M] Space works with the Mixer, Transport or Sample tab focused.
- [ ] D4 [M] After clicking Stop with the mouse, Space starts playback (it does not press Stop again).
- [ ] D5 [M] With a Sequencer step focused, Space plays or pauses and the step does not toggle; Enter still toggles it.
- [ ] D6 [M] Holding Space down toggles once.
- [ ] D7 [M] During a recording, Space stops playback and the take appears on its track.

### Things that must never happen (relevant here)

- [ ] X5 [M] Space starts or stops playback while typing in a text field, the tempo field, a rename field, search or a drop-down list.
