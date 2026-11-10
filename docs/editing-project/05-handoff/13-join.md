# Handoff brief — M13 — Join

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

**Goal:** Shift+J and the menu's Join, for sample clips (heal or bounce) and pattern clips (new pattern) (§14).

**Work**

- Engine: decide heal or bounce; heal merges windows; bounce renders the selected clips on one track into one file (gaps silent, overlaps summed) using M11's writer and rules. Pattern join builds a new pattern from the clips' windows on the first clip's sixteenth-note grid, rounds its length up to a bar, names and places it, and replaces the clips with one windowed clip. Patterns may be any whole number of bars. Every form is one undo step.
- Frontend: the Shift+J handler, Edit menu and clip menu items, refusals in the notice banner, the joined clip selected. Sequencer: the bar keys show none lit for other lengths.

**Files:** `engine/src/bounce.rs`, `engine/src/daw/daw_core/audiograph.rs`, `sequencer.rs` (`check_bars`, new-pattern-from-steps), `engine/src/sequencer_commands.rs`, `engine/src/event_dispatch.rs`, `engine/src/history.rs`, `engine/src/main.rs`; `src/views/playlist/PlaylistCanvas.tsx`, `src/util/keys.ts`, `src/state/slices/playlistSlice.ts`, `sequencerSlice.ts`, `src/views/sequencer/PatternStrip.tsx`, `src/types/sequencer.ts`

**Risks**

- Lifting the 1 / 2 / 4 bar limit touches the Sequencer grid, which may assume those lengths when laying out steps; I check it draws a 3-bar and an 8-bar pattern before relying on it.
- "Line up" for a heal must mean the result plays identically: same file, same gain, pan and pitch, touching in the source and on the timeline, no fade-out on the left piece and no fade-in on the right. Anything else bounces.

**Tests (numeric where audio)**

- Heal: cut a clip, join the halves; the result renders identically to the original and no file is written.
- Bounce: two different samples with a gap; the joined clip's render equals the sum of the two originals' renders over the span, within one 16-bit step; overlap case likewise.
- Refusals: two tracks; mixed kinds; pattern clips off the sixteenth grid.
- Pattern join: two 1-bar pattern clips a bar apart give a 3-bar pattern with an empty middle bar; the new clip schedules the same hits at the same times as the two originals did; originals untouched; name is "a + b"; undo removes the pattern and restores both clips.

- [ ] Approved

## Spec excerpts

### 14. Join

#### 14.1 General

- Shift+J joins the selection into one clip (Q28, Q29). It works on two or more clips (Q29).
- All selected clips must be on one track; otherwise nothing happens and the notice banner says so (Q142, F26, F37).
- A selection holding both sample clips and pattern clips is not joined; the notice banner says Join needs clips of one kind (Q146, F37).
- The joined clip becomes the selection (H1).
- Join is one undo step (Q14).

#### 14.2 Sample clips

- If the pieces are neighbouring windows on the same file that still line up, they are rejoined into one clip and no file is written. Otherwise the clips are rendered together into one new file in `clips/` and replaced by one clip (Q141).
- In a bounced join the new clip runs from the earliest start to the latest end. Gaps are silence in the file. Overlapping clips are summed (Q143).
- A bounced join follows the Deep copy rules for format, what is baked, gain clipping, the Save-dialog-first rule, failure handling and undo (Q144).
- Name: `<source name>_join_<n>.wav`, after the first clip's source (Q147).

#### 14.3 Pattern clips

- Join makes a new pattern holding the selected clips' steps in sequence and replaces the clips with one clip of it. The original patterns and any other clips of them are untouched (Q145, F26).
- Two halves of a cut pattern clip that still line up are rejoined into one clip of the same pattern, and no new pattern is made (F19).
- The new pattern starts where the first clip starts. Its length is the span rounded up to a whole bar; gaps are empty steps. The new clip is trimmed to the exact span, so nothing moves on the timeline (F23).
- Where the clips overlap, their steps are merged (F26).
- A pattern may be any whole number of bars. The Sequencer's 1 / 2 / 4 keys stay; on a longer pattern none is lit, and pressing one sets that length as now (F22).
- If the clips do not sit on the same sixteenth-note grid as the first clip, the join is refused and the notice banner says why (F24, F37).
- Name: the joined patterns' names in order, joined with " + ". Unnamed patterns use their numbers. More than three names become, for example, "verse + 3 more" (F25).
- The new pattern is placed after the first source pattern, in the same folder (F26).
- Undo removes the new pattern and restores the clips (F26).

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

### 11. Deep copy

#### 11.1 Command

- "Deep copy" is an item in the clip's context menu, with that exact label and no shortcut (Q122). It is offered on every clip it applies to (Q102), including pattern clips (Q26).
- It replaces the clip in place: the same clip, at the same position and track, now plays the new file (Q103).
- Chosen on a clip inside a selection, it applies to every selected clip. Each gets its own new file. It is one undo step, and if any file fails, no clip is changed (Q150, G2).

#### 11.2 What the file contains

- The clip as it plays: trim, pitch and speed, fades, pan and gain are all baked in. The track's mixer strip and the master are not (Q104).
- Samples above full scale are clipped to full scale in the file (Q105).
- Afterwards the clip's settings are neutral (no trim, Pitch 0, no fades, centre pan, 0 dB), so nothing is applied twice (Q104).
- The file is exactly as long as the clip's played length, with no added silence or tail (Q121).
- Format: WAV, 32-bit float (Q108), always stereo (Q109), at the engine's current output rate, or the source file's rate if no output device is open (Q107).

#### 11.3 Accuracy

Played back, the copy has the same number of frames as the original clip, and every sample is within one 16-bit step (about 0.00003 of full scale) of the original, except samples the original pushes above full scale, which the copy clips (Q106).

#### 11.4 Files

- Files go in `clips/` inside the project folder. For an older project that is a bare `.dawesome` file, `clips/` sits beside that file (Q114).
- Name: `<source name>_copy_<n>.wav` (Q110). The next free number is used, names are compared without regard to case, and no existing file is ever overwritten (Q111).
- The old source file is never deleted or moved (Q112).
- In a project that has never been saved, the Save dialog opens first. The deep copy happens after the save and is abandoned if you cancel (Q113).
- Save As copies `clips/` into the new project folder, and the new project's clips point at the copies (Q115).
- The project is not saved automatically afterwards (Q120).

#### 11.5 Feedback

- If writing takes longer than about half a second, the mouse shows the wait cursor until it finishes. Otherwise nothing is shown (Q117).
- On any failure the clip is left exactly as it was, any partly written file is removed, and the reason is shown in the notice banner in its error style (Q118, F37).
- An edit that lands while the clip is sounding is heard from its next start (Q63).
- A one-minute clip is written in under 2 seconds on your Mac (Q164).

#### 11.6 Undo

Undo puts the clip back on its original file with its original trim and settings. The new file stays on disk. Redo reuses that file and does not write another (Q119).

#### 11.7 Pattern clips

- The clip is rendered as it plays at that moment: current steps, row volumes applied, muted rows silent, at the current tempo (F21).
- The file is exactly the clip's length; any hit still ringing at the end is faded out over 10 ms (F20).
- Name: `<pattern name>_copy_<n>.wav` (F21).
- Afterwards the clip is an ordinary sample clip, no longer linked to the pattern. Undo turns it back into the pattern clip (F21).

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

### 19. Messages

All short messages from this project appear in the notice banner: info style for refusals (paste or repeat past the end, join needs one track, join needs one kind, join clips not on the same grid), error style for failures (deep copy or join failed) (F37).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### M. Join (M13)

- [ ] M1 [E] Cut a clip and join the halves: one clip that renders identically to the original, and no file is written.
- [ ] M2 [E] Join two different samples with a one-beat gap on one track: one clip of `<first>_join_1.wav` whose render equals the two originals' summed render over the span, within one 16-bit step.
- [ ] M3 [M] Shift+J with clips on two tracks does nothing and the notice banner says Join needs one track.
- [ ] M4 [M] Shift+J with a sample clip and a pattern clip selected does nothing and the banner says Join needs one kind of clip.
- [ ] M5 [E] Join two 1-bar pattern clips with a bar between them: one clip of a new 3-bar pattern named "a + b", whose hits fall at the same times as before; the patterns a and b are unchanged.
- [ ] M6 [M] The Sequencer shows the 3-bar pattern with none of the 1 / 2 / 4 keys lit.
- [ ] M7 [M] Pattern clips offset by less than a sixteenth note from each other are not joined, and the banner says why.
- [ ] M8 [M] After a join the joined clip is selected. Undo restores the original clips (and removes a new pattern).
- [ ] M9 [M] Join appears in a clip's menu and the Edit menu, and does the same as Shift+J.

### J. Context menus (M10, M11, M13)

- [ ] J1 [M] A sample clip's menu reads: Cut, Copy, Paste, Repeat; divider; Split at playhead, Join, Deep copy; divider; Delete. Paste is absent with an empty clipboard, Join with fewer than two clips selected, Split when the playhead does not cross the clip.
- [ ] J2 [M] A pattern clip's menu has "Explode into tracks" and "Open in Sequencer" above those.
- [ ] J3 [M] Right-clicking one of three selected clips and choosing Delete removes all three.
- [ ] J4 [M] Right-clicking an unselected clip leaves the selection as it was, and Delete removes only that clip.
- [ ] J5 [M] Right-clicking empty space at bar 5 on track 2 with clips on the clipboard opens a menu with Paste; choosing it puts the earliest clip at bar 5 with the topmost copied clip on track 2. With an empty clipboard no menu opens.
- [ ] J6 [M] The menu looks as it did, apart from the new items and divider lines.

### Things that must never happen (relevant here)

- [ ] X2 [E] A deep copy or join writes a file anywhere outside the project's folder.
- [ ] X3 [E] A deep copy or join overwrites an existing file.
- [ ] X4 [E] Undo, redo, deep copy or join deletes or moves an audio file.
- [ ] X8 [E] A failed deep copy or join leaves a clip pointing at a missing or partly written file.
