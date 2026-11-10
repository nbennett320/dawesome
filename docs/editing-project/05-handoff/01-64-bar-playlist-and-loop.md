# Handoff brief — M1 — 64-bar playlist and loop

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

**Goal:** the playlist is 64 bars; the loop ends at the last clip; the view opens on the first 4 bars (§2.1, §2.2).

**Work**

- Engine: playlist length becomes 64 bars (256 beats) everywhere it is used for clamping and seeking. The loop length becomes "end of the last clip, rounded up to a bar; 4 bars when empty". The loop's end is frozen for the length of a recording. The transport report gains the loop's end.
- Frontend: the canvas maps beats to pixels from a "beats visible at zoom 1" value of 16 instead of the playlist length, and allows zooming out to all 256 beats. Beat lines and ruler numbers thin out when zoomed out. A loop-end marker is drawn on the ruler while Loop is on. New Project and Open reset the view.

**Files**

- `engine/src/daw/state/defaults.rs`, `engine/src/daw/state/playlist.rs` (`loop_len`), `engine/src/daw/daw.rs` (`start_playlist_after`, `run_playlist`, `seek_playlist`), `engine/src/event_dispatch.rs` (`beat_to_offset`, `PlaylistTransport`, `get_playlist_data`), `engine/src/recording.rs`
- `src/views/playlist/sketch/index.ts` (zoom limits, grid), `Timeline.ts` (ruler numbers, loop marker), `theme.ts`, `src/types/transport.ts`, `src/views/transport/tape.ts`, `src/state/slices/playlistSlice.ts`

**Risks**

- Several places assume "zoom 1 shows the whole playlist" (`#zoomBy` keeps scale at 1 or more; `beatToX` divides by `beatsDisplayed`). Minimum zoom has to become a fraction.
- The Transport tab's tape maths reads `loopBeats` and `beatsDisplayed` (`tape.ts`); it must keep working with the new values.
- Recording uses the loop length to end takes (`recording.rs`).

**Tests**

- Engine: loop length for an empty playlist, for clips ending mid-bar, and frozen during a recording; a clip can be placed and moved to bar 60; seek clamps at bar 64.
- Frontend: grid-thinning function (which lines and numbers are drawn at a given pixels-per-beat); beat-to-pixel maths at fractional zoom; existing `tape.spec.ts` still passes.

- [ ] Approved

## Spec excerpts

### 2. Timeline groundwork

#### 2.1 Playlist length

- The playlist is 64 bars long (Q7, Q7a). A clip's start is kept within it, as it is kept within 4 bars today.
- A clip's right end may hang past the playlist's last beat (Q58).
- At launch, and after New Project or Open, the Playlist shows the first 4 bars, scrolled to the start (F3, H3). You can scroll right and zoom out as far as the whole 64 bars (F3).
- When zoomed out, beat lines are hidden once they would be closer than about 8 px, bar lines stay, and ruler numbers are thinned so they never overlap (F4).

#### 2.2 Loop

- With Loop on, the loop runs from the start to the end of the last clip, rounded up to the next bar. An empty playlist loops 4 bars (F2).
- While Loop is on, a small marker on the ruler shows the loop's end, in an existing theme colour (F39).
- While a recording is in progress, the loop's end stays where it was when the recording started (F40).

#### 2.3 Tempo

- When the tempo changes, every clip keeps its beat position, in the engine and on the canvas (Q8). The playhead keeps its beat too (F6).
- A sample clip's length is an amount of audio. A tempo change does not change what a trimmed or stretched clip plays or how long it lasts in seconds (Q9, Q88).
- If the playlist is playing when the tempo changes, playback carries on from the beat it had reached at the new tempo, with a very short fade (F5).
- The tempo cannot be changed while a recording is in progress (F12).

### 20. Quality bars

- **Performance:** with 500 clips in the project, edge drags, cut previews and lassos stay at 50 frames per second or better on your Mac, and every edit command returns within 50 ms (Q163).
- **Look and feel:** no existing style changes. New canvas marks use colours already in `theme.ts`. New controls are built from existing four-track and sidebar components. Cursors are standard, except the razor blade (Q165).
- **Platform:** verified on macOS only (Q4).

### 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).

## Acceptance items for this milestone

`[E]` engine test, `[U]` frontend unit test, `[M]` checked by hand in the running app by the user. Items tagged with another milestone in their group heading are not yours.

### A. Timeline groundwork (M1, M2)

- [ ] A1 [E] A clip can be placed and moved so that it starts at bar 60. A move aimed past the end is clamped to the end of bar 64.
- [ ] A2 [M] At launch the Playlist shows bars 1 to 4 across its width, as it does today.
- [ ] A3 [M] Zooming out all the way shows 64 bars. Beat lines disappear before they smear together; bar lines and non-overlapping bar numbers remain.
- [ ] A4 [E] With Loop on and one clip ending at beat 10, the loop length is 12 beats (3 bars). With no clips it is 16 beats.
- [ ] A5 [M] With Loop on, a marker on the ruler sits at the loop's end and moves when the last clip is moved or trimmed. With Loop off it is not drawn.
- [ ] A6 [E] During a recording the loop length does not change when clips are added.
- [ ] A7 [M] After Open and after New Project the view is back on bars 1 to 4.
- [ ] A8 [E] A clip at beat 8 at 120 bpm is still at beat 8 after the tempo is set to 90, and its first rendered frame is at the frame for beat 8 at 90 bpm.
- [ ] A9 [M] After changing the tempo, each clip sits on the same grid line as before and plays on that beat, without saving and reloading.
- [ ] A10 [E] A trimmed 1.000-second clip is still 1.000 second long after a tempo change.
- [ ] A11 [M] Changing the tempo during playback: playback continues from the same beat at the new tempo.
- [ ] A12 [M] While recording, the tempo field cannot be changed.
