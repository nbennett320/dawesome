# Handoff brief — M9 — Playlist tray, tools, Cut

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

**Goal:** the tray above the ruler with Select and Cut keys and the snap controls; C and S; the Cut tool; "Split at playhead" (§6, §10).

**Work**

- Tray component inside the Playlist pane; Snap key and snap-value drop-down moved out of the topbar; two new icons; the razor-blade cursor (SVG, shown to you before it ships).
- Tool state on the renderer and in Redux (for the keys' LEDs); C and S handlers.
- Cut tool: hover line at the snapped position, release-to-cut, multi-clip rule, minimum-length rule. "Split at playhead" menu item. Both call the split command from M4 as one undo step.

**Files**

- New: `src/views/playlist/PlaylistTray.tsx`, `src/components/icons/SelectToolIcon.tsx`, `CutToolIcon.tsx`, `public/cursors/razor.svg`, `src/views/playlist/sketch/cut.ts` with `cut.spec.ts`
- `src/views/playlist/Playlist.tsx`, `PlaylistCanvas.tsx`, `sketch/index.ts`, `src/components/topbar/ButtonArea.tsx`, `src/state/slices/playlistSlice.ts` or `windowSlice.ts`, `src/util/keys.ts`

**Risks**

- The canvas fills its pane and measures itself with a `ResizeObserver`; the tray takes height from it.
- Custom cursors in the macOS webview need a small image with an explicit hot spot; if it will not render, the crosshair fallback applies and I tell you.

**Tests:** `cut.spec.ts`: cut position with grid, playhead and Option; refusal inside the minimum length; which clips a click cuts given a selection. Key predicate tests for C and S (modifiers, focus, typing). Engine split tests exist from M4; add "Split at playhead" at an unsnapped position.

- [ ] Approved

## Spec excerpts

### 6. Playlist tray and tools

#### 6.1 Tray

- A strip above the ruler, inside the Playlist pane; the canvas starts below it (Q32, Q32a).
- From the left: the Select and Cut tool keys, a gap, then the Snap key and the snap-value drop-down. It uses the `ft-tray` style and `LatchKey`s (F36).
- The Snap key and snap-value drop-down move here from the topbar. The topbar keeps Metronome, Count-in and Loop (Q32b, F36).
- Each tool key is a `LatchKey` with an icon: an arrow for Select, a razor blade for Cut. The active tool's LED is lit (Q33).

#### 6.2 Tools

- There are exactly two tools: Select (the default) and Cut. Trim and stretch are edge drags inside the Select tool (Q30).
- The tool is Select at launch and after opening a project. It is not saved (Q34).
- S switches to Select and C switches to Cut, when pressed with no modifier while Playlist-focused. Caps Lock makes no difference (Q38).
- Cut is sticky: it stays until S is pressed (Q35). Escape does not leave the Cut tool; it only clears the selection (Q36).

#### 6.3 Select tool

Everything the mouse does today stays: a plain drag on empty space pans, a plain drag on a clip moves it, Shift selects, right-click opens the menu, double-click opens the clip (Q31). A plain click does not select a clip (Q44). Trackpad scrolling and pinch are unchanged (Q46). Added: edge drags (sections 8 and 9) and the lasso change (section 7).

#### 6.4 Cut tool

- Clicking a clip cuts it (section 10). The clip does not need to be selected (Q36).
- Dragging empty space pans, the wheel scrolls and zooms, right-click opens the menu, and Shift selects, all as in the Select tool. Clips cannot be moved or trimmed (Q37).

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

### H. Playlist tray, tools, Cut (M9)

- [ ] H1 [M] A tray above the ruler holds the Select and Cut keys, then the Snap key and snap-value drop-down. The topbar no longer shows the snap controls and still shows Metronome, Count-in and Loop.
- [ ] H2 [M] C lights the Cut key and S lights the Select key, with the Playlist focused. Cmd+S still saves. Neither key does anything while typing or with another view focused.
- [ ] H3 [M] After a cut the tool is still Cut. Escape clears the selection and the tool is still Cut. After relaunch the tool is Select.
- [ ] H4 [M] In the Cut tool the cursor is the razor blade over the Playlist, and a dashed sky line on the hovered clip follows the snapped mouse position.
- [ ] H5 [M] Clicking an unselected clip at beat 3 cuts it at beat 3 into two clips.
- [ ] H6 [E] The two halves of a cut, played back, produce exactly the frames of the uncut clip, at pitch −7, 0 and +5, for 44.1 kHz and 48 kHz files.
- [ ] H7 [E] After a cut, the left half has the fade-in and no fade-out; the right half has the fade-out and no fade-in; both have the original gain, pan and pitch.
- [ ] H8 [M] Clicking a clip that is part of a selection cuts every selected clip crossing that position, and no unselected clip.
- [ ] H9 [M] Pressing on a clip, moving 20 px, and releasing makes no cut. Dragging empty space pans.
- [ ] H10 [U] A click within 10 ms of a clip's edge makes no cut and shows no line.
- [ ] H11 [M] With Option held the cut lands exactly at the mouse. Stopped, with the mouse nearer the playhead than either grid line, it lands on the playhead.
- [ ] H12 [M] "Split at playhead" appears in the menu of a clip the playhead crosses, splits it there, and is absent for a clip it does not cross.
- [ ] H13 [M] A cut is one undo step; undo gives back one clip with the original id selected.
- [ ] H14 [M] The Cut tool cuts pattern clips; both halves stay linked to the pattern.

### Things that must never happen (relevant here)

- [ ] X6 [M] C, S, Shift+J, Cmd+C, Cmd+X, Cmd+V, Cmd+R or Cmd+A changes the project while typing in a text field.
- [ ] X12 [E] A paste, repeat or multi-clip cut applies to only some of its clips after a failure (each is all or nothing).
- [ ] X13 [M] A trim, cut or stretch moves a clip the user did not select or grab.
