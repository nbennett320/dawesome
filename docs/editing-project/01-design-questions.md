# 01 — Design questions, batch 1

Written 2026-11-10. Background for every "Context" line is in `00-codebase-map.md`.

> **Answered in chat, not in this file.** You asked to answer as a questionnaire in the conversation, so the boxes below are still unticked. Your answers are recorded in `01-answers.md`, and the follow-up round is in `01b-follow-ups.md`.

**How to answer**

- Tick one box per question unless it says "tick all that apply".
- A question titled "Confirm" has one box. Tick it if I understood correctly, or write the correction on the notes line.
- Write anything else on the "Other / my notes" line. A note overrides a tick.
- Every box is unticked. My pick is on the "Your recommendation" line of each question.
- Some questions say "Skip if …". Leave those blank when the condition applies.

**What is not in this batch, and why**

These depend on answers below, so they come in `01b-…` after I read this file:

- How trim, cut, stretch, deep copy and join behave on **pattern clips**, for whichever of them you switch on in Q26.
- Follow-ups on **playlist length** (loop length, default zoom) if you change it in Q7.
- Follow-ups on **tempo** (what happens mid-playback) if you choose the fix in Q8.
- What happens to the **undo history** when something outside it changes the project, once Q10 and Q11 set its scope.
- **Stretch processing** details (wait indicator, quality settings) once Q72–Q74 pick the approach.
- Details of **joining pattern clips** once Q145 picks the meaning.

**Contents**

| Part | Topic | Questions |
|---|---|---|
| 0 | How we work | Q1–Q6 |
| 1 | Foundations the code forces | Q7–Q20 |
| 2 | Data model | Q21–Q27 |
| 3 | Tools and selection | Q28–Q46 |
| 4 | Edge drag: trim | Q47–Q71 |
| 5 | Edge drag with Shift: stretch | Q72–Q89 |
| 6 | Cut tool | Q90–Q101 |
| 7 | Deep copy | Q102–Q122 |
| 8 | Clipboard and Repeat | Q123–Q140 |
| 9 | Join | Q141–Q147 |
| 10 | Context menu | Q148–Q153 |
| 11 | Space | Q154–Q159 |
| 12 | Shortcuts and cross-cutting | Q160–Q165 |

---

## Part 0 — How we work

### Q1 — Who commits?
Context: In earlier sessions you told me never to commit, because you review and commit each change yourself. The brief for this project says "Commit at the end of each milestone". These conflict, so I am asking instead of picking one.

- [ ] A. I commit at the end of each milestone, as the brief says
- [ ] B. I never commit; I leave each milestone in the working tree for you to review and commit
- [ ] C. I commit a milestone only after you have looked at it and said go
- [ ] Other / my notes:

Your recommendation: A, because the brief is newer and specific to this project, and per-milestone commits give the verifier and any later model a clean base to diff against.

### Q2 — Which branch?
Context: The repo is on `playlist-enhancements`, which currently points at the same commit as the last merge to `main`.

- [ ] A. Do all of this project's work on `playlist-enhancements`
- [ ] B. A new branch for this project (name it in the notes)
- [ ] C. One branch per milestone
- [ ] Other / my notes:

Your recommendation: A, because the branch name already matches the work and the brief says not to create extra branches.

### Q3 — Confirm: your uncommitted `docs/todo.md` stays yours
Context: `docs/todo.md` has an uncommitted change that I did not make.

- [ ] Confirm: I never edit, stage or commit `docs/todo.md`; my commits contain only files I changed for this project
- [ ] Other / my notes:

Your recommendation: Confirm, because it is your working file.

### Q4 — Which platforms do I verify on?
Context: Development happens on this Mac. CI runs tests on Ubuntu only. The engine may not compile on Linux at all (`AudioNode` is switched off there, `audiograph.rs:173`), and nothing is tested on Windows.

- [ ] A. macOS only. I write Cmd/Ctrl handling so other platforms should work, but I test nothing there
- [ ] B. macOS, and the tests must also pass in Linux CI
- [ ] C. macOS, Linux and Windows must all be verified
- [ ] Other / my notes:

Your recommendation: A, because it is the only platform I can run the app on here; B is a reasonable addition if CI is green today, which I could not check.

### Q5 — What counts as "passing" for a milestone, given today's baseline?
Context: Today `pnpm ui:test` passes (178) and `cargo test` passes (194, 5 ignored). `eslint src` already reports 3 errors and 55 warnings, and `tsc --noEmit` already reports one configuration error.

- [ ] A. Both test suites pass, and my changes add no new eslint or tsc problems. The existing problems stay (they are listed in `later.md`)
- [ ] B. I first clean the lint and typecheck baseline as a groundwork milestone, then hold every milestone to zero problems
- [ ] C. Tests only; ignore lint and typecheck
- [ ] Other / my notes:

Your recommendation: A, because cleaning the baseline is outside this project's scope and A still stops me adding to it.

### Q6 — How are mouse gestures on the canvas verified?
Context: No test can drive the Playlist canvas today. Component rendering does not work under the current Jest setup, there is no end-to-end harness, and Tauri's WebDriver tool does not support macOS. The existing pattern is to keep canvas maths in p5-free modules and unit-test those (`sketch/selection.ts`).

- [ ] A. Unit tests on extracted logic (hit zones, snapping, trim and cut maths), numeric engine tests for audio, and a manual checklist that you run in the app at each milestone
- [ ] B. A, plus I add a browser test harness (Playwright against the Vite dev server with the engine mocked) so gestures on the real canvas are automated
- [ ] C. A, plus I drive the real app myself with screen control at each milestone and report what I saw
- [ ] Other / my notes:

Your recommendation: A, because it follows the pattern the repo already uses and adds no new tooling. B is the option to tick if you want gestures regression-tested without you; it is a sizeable groundwork milestone.

---

## Part 1 — Foundations the code forces

### Q7 — The playlist is 16 beats long. Does that change?
Context: The playlist length is fixed at 16 beats (4 bars) and every clip start is clamped into it (`state/defaults.rs`, `engine/src/app/workspaces/playlist.rs:8-27`). The loop is the same 16 beats. With this limit, Paste and Repeat run out of room after a few presses.

- [ ] A. Keep 16 beats for this project. Paste, Repeat and trim obey the limit
- [ ] B. A longer fixed length (write the number of bars in the notes)
- [ ] C. The playlist grows automatically to fit its clips
- [ ] D. Playlist length becomes a project setting you can edit
- [ ] Other / my notes:

Your recommendation: B, because it is the smallest change that makes Paste and Repeat useful. It is still a scope addition, and B, C and D all bring follow-up questions (loop length, default zoom) in the next batch.

### Q8 — Should a clip stay on its beat when the tempo changes?
Context: The engine stores each clip's start in seconds; the canvas stores it in beats. Changing the tempo updates neither, so afterwards the canvas shows the clip on its old beat while the engine plays it at its old time. They agree again only after save and reload. I found this by reading the code and have not reproduced it in the app. Trim, cut and paste positions all need one consistent unit.

- [ ] A. Fix it as groundwork: clips keep their beat position when the tempo changes, in the engine and on the canvas
- [ ] B. Leave tempo behaviour alone; record it in `later.md`. New features assume the tempo does not change mid-session
- [ ] Other / my notes:

Your recommendation: A, because every snapping rule in this project is defined in beats, and with B any acceptance test that touches tempo has two right answers.

### Q9 — Confirm: a trimmed clip's length is an amount of audio, not a number of beats
Context: A sample clip's length today is its file's length in seconds (after pitch), so it covers more beats at a faster tempo.

- [ ] Confirm: after a tempo change, a trimmed clip plays exactly the same audio as before; only its start beat is tied to the tempo (subject to Q8)
- [ ] Other / my notes:

Your recommendation: Confirm, because audio clips already behave this way.

### Q10 — Undo/redo does not exist. Does this project build it?
Context: There is no undo, redo or history anywhere in the app. The brief asks for undo granularity for every operation, which assumes it exists.

- [ ] A. No undo in this project
- [ ] B. Build undo/redo for Playlist clip operations only (the new ones, plus the existing ones you tick in Q11)
- [ ] C. Build undo/redo for the whole project (clips, clip settings, mixer, sequencer, tempo)
- [ ] Other / my notes:

Your recommendation: B, because destructive-feeling edits (cut, trim, join, Cmd+X) need a way back, and C roughly doubles the project by pulling in the mixer and sequencer.

### Q11 — Which existing operations join the undo history? (tick all that apply)
Context: Skip if Q10 is A. These operations exist today and are not undoable. Clip-setting knobs send many small updates per drag (`playlistSlice.ts:592-626`).

- [ ] A. Adding a clip by dragging it in from the sidebar or Sequencer
- [ ] B. Moving clips (one or a selection)
- [ ] C. Deleting clips
- [ ] D. Clip settings from the Sample tab (gain, pan, pitch, attack, decay)
- [ ] E. Exploding a pattern clip (this also creates patterns in the Sequencer)
- [ ] F. Clips placed by a finished recording
- [ ] Other / my notes:

Your recommendation: A, B and C, because they are simple clip operations and leaving them out would make undo feel broken. D needs gesture grouping for the knobs, and E and F change things outside the playlist (patterns, files on disk).

### Q12 — Where does the undo history live?
Context: Skip if Q10 is A. The engine owns the project state and assigns clip ids; Redux mirrors it.

- [ ] A. In the engine (Rust): each edit records how to reverse itself, and the frontend asks the engine to undo or redo, then redraws from what the engine returns
- [ ] B. In the frontend (Redux): the frontend remembers reverse commands and replays them to the engine
- [ ] Other / my notes:

Your recommendation: A, because the engine is the source of truth, it can restore a deleted clip with its original id, and the reversal logic can be tested numerically in `cargo test`.

### Q13 — Undo and redo shortcuts
Context: Skip if Q10 is A. Nothing is bound to these keys today.

- [ ] A. Cmd+Z undo, Shift+Cmd+Z redo (Ctrl on other platforms)
- [ ] B. A, plus Ctrl+Y as a second redo on non-Mac platforms
- [ ] Other / my notes:

Your recommendation: A, because it is the macOS convention and macOS is the tested platform.

### Q14 — Confirm: one gesture is one undo step
Context: Skip if Q10 is A.

| Action | Undo steps |
|---|---|
| One edge drag (press to release), trim or stretch | 1 |
| One cut click, however many clips it cuts | 1 |
| One Paste, Cmd+X or Repeat, however many clips | 1 |
| One Deep copy | 1 |
| One Join | 1 |
| One clip or selection move | 1 |
| Deleting a selection | 1 |
| Dropping one clip in from the sidebar | 1 |

- [ ] Confirm: the table is right
- [ ] Other / my notes:

Your recommendation: Confirm, because a step should match what your hand did.

### Q15 — How many undo steps are kept?
Context: Skip if Q10 is A.

- [ ] A. 100
- [ ] B. 50
- [ ] C. No limit for the session
- [ ] Other / my notes:

Your recommendation: A, because steps are small records and 100 covers a long editing run.

### Q16 — Confirm: the history's lifetime
Context: Skip if Q10 is A.

- [ ] Confirm: the history is cleared by New Project and Open, survives Save, and is not written into the project file
- [ ] Other / my notes:

Your recommendation: Confirm, because clip ids change on every load, so a saved history could not be replayed.

### Q17 — Confirm: what is not an undo step
Context: Skip if Q10 is A.

- [ ] Confirm: changing the selection, switching tools, zooming, scrolling, moving the playhead, and play/stop are never undo steps
- [ ] Other / my notes:

Your recommendation: Confirm, because none of them change the project.

### Q18 — What is selected after an undo or redo?
Context: Skip if Q10 is A. The selection is not part of the project state (`sketch/index.ts:263`).

- [ ] A. The clips the undo or redo just changed or restored become the selection
- [ ] B. The selection is cleared
- [ ] C. The selection is left as it was, minus clips that no longer exist
- [ ] Other / my notes:

Your recommendation: A, because it shows you what just changed.

### Q19 — When do Cmd+Z and Shift+Cmd+Z act on the project?
Context: Skip if Q10 is A. Existing playlist shortcuts work only when the Playlist is the focused view and you are not typing (`PlaylistCanvas.tsx:172-197`).

- [ ] A. Whenever you are not typing in a text field, whichever view is focused. In a text field they undo typing
- [ ] B. Only when the Playlist is the focused view and you are not typing
- [ ] Other / my notes:

Your recommendation: A, because the history belongs to the project, and an undo that silently does nothing while the Mixer is focused is confusing.

### Q20 — Confirm: no "unsaved changes" indicator in this project
Context: The app has no dirty flag; it asks before replacing a project whenever the playlist has clips (`projectSlice.ts:99-108`).

- [ ] Confirm: this project adds no unsaved-changes tracking; it stays in `later.md`
- [ ] Other / my notes:

Your recommendation: Confirm, because none of the seven goals needs it.

---

## Part 2 — Data model

### Q21 — Confirm: "same source" means "same audio file"
Context: Clips refer to audio only by file path (`audiograph.rs:178`). There is no shared source object, and each clip loads its own copy of the file.

- [ ] Confirm: two clips are "instances of the same source" when they point at the same file on disk; there is no other link between them
- [ ] Other / my notes:

Your recommendation: Confirm, because it is the only definition the current model supports, and it is all Deep copy needs.

### Q22 — Confirm: trimming never changes the audio file
Context: Today a clip always plays its whole file (`audiograph.rs:298-300`).

- [ ] Confirm: a trimmed clip is the same file plus two new per-clip values, where in the file it starts and how much of the file it plays. The file is never rewritten, and a trim can always be dragged back out
- [ ] Other / my notes:

Your recommendation: Confirm, because it is what makes Cut possible (two clips with different windows on one file) and what Deep copy then bakes.

### Q23 — Confirm: clips never affect each other
Context: Per-clip settings are already independent per clip (`ClipSettings` on each `AudioNode`).

- [ ] Confirm: trimming, stretching, cutting or changing settings on one clip never changes any other clip, including clips of the same file and the other half of a cut
- [ ] Other / my notes:

Your recommendation: Confirm, because it matches today's settings and your failure criterion "editing one instance silently changes another".

### Q24 — Project file version
Context: The format is at version 2. Builds refuse files with a higher version than they know. Today a project is written at the lowest version that can represent it, so a project without patterns is still written as version 1 (`project.rs:275-277`).

- [ ] A. Follow that convention: a project is written as version 3 only if some clip is trimmed or stretched; otherwise as today. Older builds refuse version 3 files with the existing "saved by a newer version" message
- [ ] B. Every project saved by the new build is version 3
- [ ] C. No version bump; older builds open new files and silently play trimmed clips untrimmed
- [ ] Other / my notes:

Your recommendation: A, because it keeps the existing convention and never lets an older build silently play the wrong audio.

### Q25 — Confirm: existing projects open unchanged
Context: There is no migration code; missing fields take defaults.

- [ ] Confirm: every existing version 1 and version 2 project opens with all clips untrimmed and unstretched, and nothing in the file is rewritten until you save
- [ ] Other / my notes:

Your recommendation: Confirm, because the new fields can default to "whole file, no stretch".

### Q26 — Which features apply to pattern clips? (tick all that apply)
Context: A pattern clip has no audio, length or settings of its own. It is a link to a Sequencer pattern and is as long as the pattern (`audiograph.rs:350-358`). Your notes say Join "works for patterns and samples". Each box you tick gets its own detailed questions in the next batch.

- [ ] A. Copy, cut (Cmd+X), paste and Repeat
- [ ] B. Trim by dragging edges
- [ ] C. The Cut tool
- [ ] D. Stretch
- [ ] E. Deep copy (which would turn the pattern clip into an audio clip)
- [ ] F. Join
- [ ] Other / my notes:

Your recommendation: A and F, because A is straightforward and F is in your notes. B and C are worth having but need a new idea of "part of a pattern"; D and E have no obvious meaning for a drum pattern.

### Q27 — When a feature does not apply to a pattern clip, what happens?
Context: The context menu already leaves out items that do not apply (a sample clip has no "Explode" item, `PlaylistCanvas.tsx:58-77`). Shift+E on a mixed selection silently skips the sample clips.

- [ ] A. Nothing: the gesture has no effect on that clip, its menu item is left out, and a mixed selection applies the action only to the clips it fits
- [ ] B. A short message explains why nothing happened
- [ ] Other / my notes:

Your recommendation: A, because it matches how Explode already treats clips it cannot act on.

---

## Part 3 — Tools and selection

### Q28 — What does "Shift mode → selection mode" mean?
Context: Holding Shift is already how selection works: Shift+click toggles a clip, Shift+drag on empty space draws the lasso, and Shift+E acts on the selection (`sketch/index.ts:1012-1016`, `PlaylistCanvas.tsx:186-197`).

- [ ] A. A held modifier, as now: while Shift is down, mouse actions select, and Shift+letter shortcuts act on the selection (Shift+J joins, Shift+E explodes)
- [ ] B. A sticky mode: tapping Shift switches a selection mode on, and it stays on until switched off
- [ ] C. "Selection mode" is just the Select tool; Shift is only a shortcut modifier
- [ ] Other / my notes:

Your recommendation: A, because it is what the code already does and Shift+E set the precedent.

### Q29 — What does the "2" mean in "Ex, 2 Shift + J = Join"?
Context: Your older note says "Ex, 2 Shift + J = Join". Your newer note says "2 things are selected with lasso already, i press Shift + J = Join, they become one".

- [ ] A. It is the example count: two clips are selected. Join works on any selection of two or more
- [ ] B. Join works on exactly two clips
- [ ] C. "2" is a key pressed before Shift+J
- [ ] Other / my notes:

Your recommendation: A, because the newer note reads that way.

### Q30 — Confirm: the starting set of tools
Context: There is no tool or mode concept in the app today.

- [ ] Confirm: this project adds exactly two tools, Select (the default) and Cut. Trim and stretch are edge drags inside the Select tool, not tools of their own
- [ ] Other / my notes:

Your recommendation: Confirm, because that is the list in your goal 7.

### Q31 — Confirm: the Select tool keeps today's mouse behaviour
Context: Today a plain drag on empty space pans, a plain drag on a clip moves it, Shift selects, right-click opens the menu, and double-click opens the clip.

- [ ] Confirm: in the Select tool all of these stay as they are, apart from the changes you choose in Q39–Q45 and the new edge drags
- [ ] Other / my notes:

Your recommendation: Confirm, because you described the default tool as "currently implemented behavior".

### Q32 — Where does the tool switcher go?
Context: The topbar has two recessed trays of keys; Snap, which only affects the Playlist, is in the second (`src/components/topbar/ButtonArea.tsx`).

- [ ] A. A third tray in the topbar, next to Snap
- [ ] B. Inside the Playlist pane, as a small tray in a corner over the canvas
- [ ] C. In the Playlist's tab bar
- [ ] Other / my notes:

Your recommendation: A, because Snap already sets the precedent of Playlist-only keys in the topbar, and the canvas has no overlay controls today.

### Q33 — What do the tool keys look like?
Context: The existing toggles are `LatchKey`s: an extra-small `AnalogButton` with a blue LED above it and an icon on the key (`LatchKey.tsx`, `SnapButton.tsx`).

- [ ] A. Two `LatchKey`s with icons (an arrow for Select, a razor blade for Cut), the active tool's LED lit
- [ ] B. Two `LatchKey`s with text ("SEL", "CUT")
- [ ] C. Icons on the keys with a small `ft-label` caption under each
- [ ] Other / my notes:

Your recommendation: A, because it matches the Snap, Loop and Metronome keys exactly and needs only two new icons in the existing icon style.

### Q34 — Is the active tool remembered?
Context: Zoom, scroll and selection are not saved anywhere today.

- [ ] A. No: the tool is Select at launch and after opening a project
- [ ] B. Saved in the project file
- [ ] C. Remembered between launches as an app preference
- [ ] Other / my notes:

Your recommendation: A, because starting in Cut by surprise costs more than pressing C.

### Q35 — How does pressing C behave?
Context: You wrote that C "engages the cut mode" and S "engages the default selection mode".

- [ ] A. Sticky: C switches to Cut and it stays until you press S (or Escape, see Q36)
- [ ] B. One shot: after one cut the tool returns to Select
- [ ] C. Spring-loaded: Cut is active only while C is held
- [ ] Other / my notes:

Your recommendation: A, because "engages" reads as a switch and S exists to switch back.

### Q36 — What does Escape do while the Cut tool is active?
Context: Escape currently clears the selection (`sketch/index.ts:1136-1141`).

- [ ] A. Returns to the Select tool and clears the selection as it does now
- [ ] B. Returns to the Select tool and leaves the selection alone
- [ ] C. Only clears the selection; the tool stays on Cut
- [ ] Other / my notes:

Your recommendation: A, because Escape as "back to normal" is predictable.

### Q37 — In the Cut tool, what do the other mouse gestures do?
Context: In the Select tool, dragging a clip moves it and dragging its edge will trim it.

- [ ] A. Clicking a clip cuts. Dragging on empty space still pans, the wheel still scrolls and zooms, right-click still opens the menu, Shift still selects. Clips cannot be moved or trimmed
- [ ] B. Clicking a clip cuts and nothing else responds except scrolling and zooming
- [ ] Other / my notes:

Your recommendation: A, because you need to pan and select between cuts without changing tools.

### Q38 — Confirm: when C and S switch tools
Context: Cmd+S is Save and Cmd+C will be Copy. Existing playlist keys work only when the Playlist is the focused view and you are not typing.

- [ ] Confirm: C and S switch tools only when pressed with no modifier, while the Playlist is the focused view, and not while typing in a text field. Caps Lock makes no difference
- [ ] Other / my notes:

Your recommendation: Confirm, because it matches Delete and Shift+E.

### Q39 — How do you start a lasso over a clip?
Context: Today a Shift-press over a clip toggles that clip and never starts a lasso; a plain drag on a clip moves it; a plain drag on empty space pans (`sketch/index.ts:1012-1038`).

- [ ] A. Shift+drag starts a lasso anywhere, including over a clip. Shift+click without moving still toggles the clip. Plain drags are unchanged
- [ ] B. A plain drag on empty space becomes the lasso (panning moves to the wheel and trackpad only), and Shift+drag starts a lasso over a clip
- [ ] C. A different modifier (write it in the notes) starts a lasso anywhere
- [ ] Other / my notes:

Your recommendation: A, because it fixes exactly what you reported and changes nothing else.

### Q40 — How far must the mouse move before a Shift-press becomes a lasso?
Context: Clip drags start after 10 px (`mouseDragDetectionThreshold`). The lasso has no threshold today because it never had to be told apart from a click.

- [ ] A. 10 px, the same as clip drags
- [ ] B. 4 px
- [ ] Other / my notes:

Your recommendation: A, because one threshold for "click or drag" everywhere is easier to learn.

### Q41 — A lasso starts over a clip. Is that clip selected?
Context: The lasso selects every clip its box touches (`selection.ts:36-56`).

- [ ] A. Yes: the box starts inside the clip, so it touches it
- [ ] B. No: the clip under the starting point is left out unless the box covers it completely
- [ ] Other / my notes:

Your recommendation: A, because it is the existing rule with no special case.

### Q42 — Does a lasso add to the selection or replace it?
Context: Today it always adds: the result is what was selected before plus everything the box touches (`selection.ts:44-54`).

- [ ] A. Always adds, as now
- [ ] B. Replaces the selection; a second modifier adds
- [ ] Other / my notes:

Your recommendation: A, because the lasso already requires Shift, which is the "add" modifier for clicks.

### Q43 — Confirm: the lasso's hit rule stays
Context: A clip is hit when the box touches any part of the clip's lane-height box, label band included.

- [ ] Confirm: unchanged
- [ ] Other / my notes:

Your recommendation: Confirm, because nothing in the goals asks for a change.

### Q44 — Should a plain click on a clip select it?
Context: Today a plain click selects nothing; pressing an unselected clip only clears the selection (`sketch/index.ts:1024`). Copying a single clip would need a Shift+click first.

- [ ] A. Yes: a plain click or press on a clip makes it the only selected clip (a press on an already-selected clip keeps the whole selection, so group drags still work)
- [ ] B. No: keep today's behaviour
- [ ] Other / my notes:

Your recommendation: A, because Cmd+C, Cmd+X and Cmd+R all act on the selection, and "click it, copy it" is what people expect. It is a change to existing behaviour.

### Q45 — Should Cmd+click (Ctrl+click elsewhere) also toggle a clip?
Context: Only Shift+click toggles today.

- [ ] A. No
- [ ] B. Yes, as a second way to do what Shift+click does
- [ ] Other / my notes:

Your recommendation: A, because it is not in your goals.

### Q46 — Confirm: trackpad and touch
Context: Two-finger scrolling pans and scrolls, pinch zooms, and there is no touch-screen handling.

- [ ] Confirm: trackpad scrolling and pinch stay as they are, and no touch-screen support is added
- [ ] Other / my notes:

Your recommendation: Confirm, because the new gestures are all mouse-button drags.

---

## Part 4 — Edge drag: trim

### Q47 — How wide is the grab zone at each edge?
Context: There are no edge zones today; the whole clip is one hit box.

- [ ] A. 6 screen pixels inside each edge, the same at every zoom level
- [ ] B. 8 screen pixels inside each edge
- [ ] C. 4 screen pixels inside each edge
- [ ] Other / my notes:

Your recommendation: A, because it is wide enough to hit and narrow enough to leave short clips movable.

### Q48 — What about clips too narrow for two zones and a middle?
Context: Zoomed out, a short clip can be a few pixels wide.

- [ ] A. Each zone shrinks to at most a third of the clip's width, so the middle third always moves the clip
- [ ] B. The edges win: a very narrow clip can be trimmed but not moved until you zoom in
- [ ] Other / my notes:

Your recommendation: A, because moving is the more common action.

### Q49 — Confirm: the zones run the clip's full height
Context: A clip is a card with a label band on top and the waveform below.

- [ ] Confirm: the edge zones cover both the label band and the body
- [ ] Other / my notes:

Your recommendation: Confirm, because a taller target is easier to hit.

### Q50 — Which cursor over an edge?
Context: The canvas uses only standard cursors today.

- [ ] A. The standard horizontal-resize cursor (`ew-resize`) on both edges
- [ ] B. The standard column-resize cursor (`col-resize`)
- [ ] C. Custom bracket cursors, one per edge (new image assets)
- [ ] Other / my notes:

Your recommendation: A, because it is the conventional signal and adds no assets.

### Q51 — Is there a visible handle when hovering an edge?
Context: Clips have no hover styling today.

- [ ] A. No; the cursor change is the only signal
- [ ] B. Yes; a 2 px bar in the sky accent colour is drawn on the hovered edge
- [ ] Other / my notes:

Your recommendation: A, because you asked for no restyling.

### Q52 — What do you see while dragging an edge to trim?
Context: Clip moves show a dashed phantom at the landing spot and commit on release (`sketch/index.ts:741-775`).

- [ ] A. The clip itself resizes live under the mouse, revealing or hiding waveform, and the engine is told once on release
- [ ] B. A dashed phantom shows the new bounds; the clip changes on release
- [ ] Other / my notes:

Your recommendation: A, because trimming is about seeing where in the waveform the edge lands.

### Q53 — What does the dragged edge snap to?
Context: With Snap on, moved clips snap their start to the snap grid on the timeline (`audiograph.rs:977-991`).

- [ ] A. The same grid: with Snap on, the edge's position on the timeline snaps to the nearest snap step
- [ ] B. Relative steps: the edge moves in whole snap steps from where it started, keeping any off-grid offset
- [ ] C. Trims never snap
- [ ] Other / my notes:

Your recommendation: A, because it is the rule moves already follow.

### Q54 — Is there a key that bypasses snap while dragging?
Context: No modifier bypasses snap today; you toggle the Snap key.

- [ ] A. No
- [ ] B. Holding Option (Alt) bypasses snap for the new gestures only (trim, stretch, cut)
- [ ] C. Holding Option bypasses snap for the new gestures and for existing clip moves and drops
- [ ] Other / my notes:

Your recommendation: B, because fine trims are the case where a temporary bypass matters most, and it leaves existing behaviour alone.

### Q55 — What is the shortest a clip can be trimmed to?
Context: Nothing limits clip length today.

- [ ] A. 10 milliseconds of played audio
- [ ] B. One snap step while Snap is on, 10 milliseconds while it is off
- [ ] C. One sample
- [ ] Other / my notes:

Your recommendation: A, because a clip must stay wide enough to find and grab, and a fixed floor is the simplest rule.

### Q56 — What happens when a trim reaches the end of the source audio?
Context: A clip cannot play audio the file does not have.

- [ ] A. It stops at the source boundary
- [ ] B. It can extend; the extra region is silence
- [ ] C. It can extend; the source loops
- [ ] Other / my notes:

Your recommendation: A, because it keeps "a clip is a window on its file" true and keeps Cut and Deep copy simple.

### Q57 — Confirm: trimming the left edge does not slide the audio
Context: Two things could happen when the left edge moves right: the audio stays where it is on the timeline and the start is hidden, or the audio slides with the edge.

- [ ] Confirm: the audio stays where it is on the timeline. Dragging the left edge right hides the start; dragging it back reveals it. The left edge stops at the source's first sample or at beat 0, whichever comes first
- [ ] Other / my notes:

Your recommendation: Confirm, because that is what "trim" means in every editor I know.

### Q58 — Can the right edge be dragged out past the end of the playlist?
Context: Clip starts are clamped into the playlist, but an untrimmed clip may already hang past its last beat.

- [ ] A. Yes, as far as the source allows
- [ ] B. No, it stops at the playlist's last beat
- [ ] Other / my notes:

Your recommendation: A, because untrimmed clips can already overhang.

### Q59 — What if a trim makes the clip overlap its neighbour?
Context: Clips can overlap on a track today, and overlapping clips all play.

- [ ] A. Overlap is allowed, as now
- [ ] B. The edge stops at the neighbouring clip
- [ ] C. The neighbour is trimmed back to make room
- [ ] Other / my notes:

Your recommendation: A, because it is the existing rule everywhere else.

### Q60 — Do the fades follow the trimmed edges?
Context: Attack fades in from the clip's start and decay fades out to its end (`clip.rs:101-131`).

- [ ] A. Yes: the fade-in always starts at the clip's current left edge and the fade-out always ends at its current right edge
- [ ] B. No: fades stay attached to the source audio, so trimming can cut into or remove a fade
- [ ] Other / my notes:

Your recommendation: A, because a fade exists to stop a click at the clip's edge, wherever that edge is.

### Q61 — A trim makes the clip shorter than its fade. What happens to the fade value?
Context: Today fades are clamped to the clip's length and the stored value is changed (`ClipSettings::clamped`, `clip.rs:66-81`).

- [ ] A. The stored fade is shortened to fit, as now. Dragging the edge back out does not restore it (undo does, if undo is built)
- [ ] B. The stored fade keeps its value and is only limited while the clip is too short for it
- [ ] Other / my notes:

Your recommendation: A, because it is the existing rule and the Sample tab then always shows the fade that actually plays.

### Q62 — Do you hear anything while dragging an edge?
Context: The engine has no scrubbing; edits are heard the next time a clip starts.

- [ ] A. No audio preview while dragging
- [ ] B. The audio under the edge is scrubbed while dragging
- [ ] Other / my notes:

Your recommendation: A, because scrubbing is a separate engine feature.

### Q63 — An edit lands while that clip is sounding. When do you hear it?
Context: Today a settings change is heard "the next time the clip starts playing" (`event_dispatch.rs:588-599`); a voice that is already sounding is not touched. This question covers trim, stretch, cut and deep copy.

- [ ] A. The same rule: the sounding pass finishes as it was, and the edit is heard from the clip's next start (next loop pass, or after stop and play)
- [ ] B. The engine restarts that clip's voice at once so the edit is heard immediately
- [ ] Other / my notes:

Your recommendation: A, because it matches every existing edit and avoids clicks from swapping a voice mid-note.

### Q64 — Several clips are selected and you drag one's edge. What trims?
Context: Dragging one selected clip already moves the whole selection.

- [ ] A. Only the clip you grabbed
- [ ] B. Every selected clip trims by the same amount, each stopping at its own limits
- [ ] C. Every selected clip trims by the same amount, and all stop together when any one reaches a limit
- [ ] Other / my notes:

Your recommendation: A, because it is predictable and half the work; B is the option to tick if you often trim stacked takes together.

### Q65 — Confirm: pressing an edge affects the selection like pressing the clip
Context: This follows whatever you choose in Q44.

- [ ] Confirm: pressing a clip's edge zone changes the selection exactly as pressing its middle would
- [ ] Other / my notes:

Your recommendation: Confirm, because one rule for "you pressed this clip" is simpler.

### Q66 — Does a trimmed clip look different?
Context: A clip whose Sample-tab settings differ from the defaults shows an amber dot in its label (`PlaylistObject.ts:187-196`).

- [ ] A. No indicator; the shorter clip and its waveform are the indication
- [ ] B. A trimmed clip shows the amber "modified" dot
- [ ] C. A small notch on each trimmed edge
- [ ] Other / my notes:

Your recommendation: A, because the amber dot means "settings changed", and the Sample tab's Reset would not remove a trim.

### Q67 — What does the Sample tab show for a trimmed clip?
Context: The Sample tab draws the clip's waveform with its fade envelope and shows its length (`src/views/sample-preview/`).

- [ ] A. Only the part the clip plays, with length and fades for that part
- [ ] B. The whole file, with the played region highlighted
- [ ] C. Unchanged: the whole file as if it were untrimmed
- [ ] Other / my notes:

Your recommendation: A, because the fades apply to the trimmed clip, and drawing them over the whole file would be wrong.

### Q68 — Confirm: no numeric trim or stretch fields
Context: The Sample tab has typed fields for the five settings.

- [ ] Confirm: trim and stretch are set only by dragging in the Playlist; no numeric fields are added
- [ ] Other / my notes:

Your recommendation: Confirm, because none were asked for.

### Q69 — Confirm: Escape cancels an edge drag
Context: Escape already cancels a lasso and a sidebar drag.

- [ ] Confirm: pressing Escape during an edge drag (trim or stretch) puts the clip back as it was and sends nothing to the engine
- [ ] Other / my notes:

Your recommendation: Confirm, because it matches the other drags.

### Q70 — You release an edge drag outside the canvas. What happens?
Context: A clip move released outside the canvas is cancelled (`sketch/index.ts:1042-1057`).

- [ ] A. The trim is applied at the edge's last position
- [ ] B. The trim is cancelled, like a move
- [ ] Other / my notes:

Your recommendation: A, because the mouse easily leaves the canvas sideways while pulling an edge near the window border, and losing the trim there would be annoying.

### Q71 — Does the view scroll sideways when you drag an edge to the canvas border?
Context: Dragging near the top or bottom auto-scrolls the tracks. There is no sideways auto-scroll for any drag (`sketch/index.ts:838-850`).

- [ ] A. No, as for moves today
- [ ] B. Yes, for edge drags only
- [ ] C. Yes, for edge drags and clip moves
- [ ] Other / my notes:

Your recommendation: A, because moves do not do it either; it goes in `later.md`.

---

## Part 5 — Edge drag with Shift: stretch

### Q72 — Does stretching keep the pitch?
Context: The existing Pitch setting already changes speed and length together, like tape (`clip.rs:50-57`). There is no pitch-preserving stretch and no library for one. FL Studio's playlist stretch keeps pitch.

- [ ] A. Pitch-preserving time-stretch: the clip gets longer or shorter and its pitch does not change
- [ ] B. Tape-style: stretching changes speed and pitch together, exactly what the Pitch knob does, set by dragging
- [ ] C. Both, chosen per clip (say in the notes which is the default)
- [ ] Other / my notes:

Your recommendation: A, because B only adds a second way to turn the Pitch knob, and "time-stretch" normally means pitch stays. A is the most expensive item in the project: a new DSP dependency, a render step, and looser numeric tolerances.

### Q73 — Which stretch algorithm or library?
Context: Skip if Q72 is B. The crate must build with rustc 1.86, and the engine crate is MIT-licensed. I have not built any of these yet; whichever you pick, I will prove it builds here before the plan is final.

- [ ] A. Signalsmith Stretch (MIT-licensed C++ library, through a Rust binding crate). Good quality on full mixes and tonal material; moderate CPU; needs a C++ compiler at build time
- [ ] B. Rubber Band (C++). Very good quality; licensed under GPL or a paid commercial licence, which conflicts with an MIT project
- [ ] C. SoundTouch (C++, LGPL). Older time-domain method; fine on drums, audible artefacts on sustained sounds
- [ ] D. The pure-Rust `timestretch` crate (phase vocoder plus overlap-add). No C++; a young library whose quality I cannot vouch for yet
- [ ] E. A small overlap-add stretcher written in this repo. No dependency; good on drums and short hits, rough on sustained or tonal material
- [ ] Other / my notes:

Your recommendation: A, because it has the best quality that fits the licence. If it does not build on rustc 1.86, my fallback would be D, and I would come back to you before switching.

### Q74 — Is stretching computed while playing, or rendered ahead?
Context: Skip if Q72 is B. The audio callback must stay cheap; it already decodes files as it plays.

- [ ] A. Rendered: when you release the drag, the engine computes the stretched audio once and keeps it in memory. Playback reads the result
- [ ] B. Real time: the stretcher runs inside playback every time the clip plays
- [ ] Other / my notes:

Your recommendation: A, because it keeps heavy DSP off the audio thread, makes Deep copy a plain copy of the same audio, and lets tests compare exact buffers.

### Q75 — What happens to stretched clips when a project is opened?
Context: Skip unless Q74 is A.

- [ ] A. They are rendered again while the project loads; nothing extra is stored on disk
- [ ] B. The rendered audio is cached as files inside the project folder and reused
- [ ] Other / my notes:

Your recommendation: A, because it adds no files to manage. Projects with many long stretched clips will open more slowly.

### Q76 — How does stretch combine with the Pitch knob?
Context: Skip if Q72 is B. Today Pitch changes speed, so +12 semitones halves the clip's length.

- [ ] A. Independent: Pitch keeps working as it does now (changing speed and length), and stretch multiplies the length on top
- [ ] B. Once a clip has been stretched, its length is locked and the Pitch knob changes pitch only
- [ ] C. Stretch replaces Pitch's effect on length for every clip: Pitch always changes pitch only, and only dragging changes length
- [ ] Other / my notes:

Your recommendation: A, because it changes nothing about existing clips and projects. C is the cleaner model, but it changes how every existing pitched clip sounds and how long it is.

### Q77 — Confirm (only if Q72 is B): stretching writes the Pitch setting
Context: Skip unless Q72 is B. Pitch and speed are one control.

- [ ] Confirm: a tape-style stretch sets the clip's Pitch value (for example to −7.02 semitones), the Sample tab shows it, and the limit is Pitch's ±24 semitones (a quarter to four times the length)
- [ ] Other / my notes:

Your recommendation: Confirm, because a separate hidden speed value would make two controls fight over one property.

### Q78 — How far can a clip be stretched?
Context: Skip if Q72 is B. Quality falls off quickly at extreme ratios.

- [ ] A. From a quarter of its length to four times its length
- [ ] B. From half to double
- [ ] C. No limit beyond the minimum clip length
- [ ] Other / my notes:

Your recommendation: A, because it matches the range Pitch already allows.

### Q79 — Confirm: the opposite edge stays put
Context: —

- [ ] Confirm: Shift+dragging the right edge keeps the clip's start fixed and puts its end at the cursor; Shift+dragging the left edge keeps its end fixed and puts its start at the cursor
- [ ] Other / my notes:

Your recommendation: Confirm, because it is the direct reading of "stretches the clip to wherever the cursor goes".

### Q80 — Confirm: stretch applies to what the clip currently plays
Context: A clip may already be trimmed.

- [ ] Confirm: stretching squeezes or expands the clip's current (trimmed) content into the new length; it does not reveal trimmed-away audio
- [ ] Other / my notes:

Your recommendation: Confirm, because the visible clip is what you grabbed.

### Q81 — Confirm: how a Shift-press over a clip is read
Context: Shift over a clip will mean three things: toggle (existing), lasso (Q39) and stretch.

- [ ] Confirm: with Shift held, a drag that starts in an edge zone stretches; a drag that starts anywhere else on the clip follows Q39; a Shift+click with no drag toggles the clip, even in an edge zone
- [ ] Other / my notes:

Your recommendation: Confirm, because the edge zones are small and you chose Shift for stretch. The cost is that a lasso cannot start on the outer few pixels of a clip.

### Q82 — You press or release Shift in the middle of an edge drag. What happens?
Context: —

- [ ] A. Nothing: the drag is a trim or a stretch according to Shift at the moment of the press
- [ ] B. The drag switches between trim and stretch live
- [ ] Other / my notes:

Your recommendation: A, because switching mid-drag makes the clip jump.

### Q83 — Confirm: stretch snaps like trim
Context: See Q53 and Q54.

- [ ] Confirm: the dragged edge follows the same snap rule and the same bypass key as a trim
- [ ] Other / my notes:

Your recommendation: Confirm, because stretching to the grid is the main use.

### Q84 — What do you see while dragging to stretch?
Context: See Q52.

- [ ] A. The clip and its waveform squeeze or expand live under the mouse; the audio is processed on release
- [ ] B. A dashed phantom shows the new bounds; the clip changes on release
- [ ] Other / my notes:

Your recommendation: A, because it should feel the same as a trim.

### Q85 — Does a stretched clip look different?
Context: See Q66.

- [ ] A. No indicator
- [ ] B. The stretch amount is shown in the label band, for example "×1.50"
- [ ] C. The amber "modified" dot
- [ ] Other / my notes:

Your recommendation: B, because unlike a trim, a stretch cannot be seen from the clip's shape once you forget its original length.

### Q86 — How do you remove a stretch?
Context: Dragging back to exactly the original length by eye is not reliable.

- [ ] A. A "Reset stretch" item in the clip's context menu, shown only on stretched clips
- [ ] B. Only by undo or by dragging back
- [ ] Other / my notes:

Your recommendation: A, because without it there is no way to get back to exactly unstretched once the history has moved on.

### Q87 — What does the Sample tab's Reset button reset?
Context: Reset currently returns the five settings to their defaults (`resetClipSettings`, `playlistSlice.ts:668`).

- [ ] A. The five settings only, as now
- [ ] B. The five settings and the stretch
- [ ] C. The five settings, the stretch and the trim
- [ ] Other / my notes:

Your recommendation: A, because Reset sits beside the five knobs and should not undo work done in the Playlist.

### Q88 — A stretched clip, then a tempo change. What happens?
Context: See Q9. You might stretch a loop to exactly two beats.

- [ ] A. The clip keeps its length in seconds, so it no longer spans exactly two beats
- [ ] B. The clip keeps its length in beats and is re-stretched to fit the new tempo
- [ ] Other / my notes:

Your recommendation: A, because it is consistent with every other audio clip. B is a tempo-sync feature of its own.

### Q89 — Several clips are selected and you Shift+drag one's edge. What stretches?
Context: See Q64.

- [ ] A. Only the clip you grabbed
- [ ] B. Every selected clip, by the same ratio
- [ ] Other / my notes:

Your recommendation: A, because it should match the answer I recommended for trim; tick B if you ticked B or C in Q64.

---

## Part 6 — Cut tool

### Q90 — How can a cut be made? (tick all that apply)
Context: You asked for a Cut tool on the C key.

- [ ] A. Cut tool: click a clip to split it at the click position
- [ ] B. Context menu item "Split at playhead" on a clip the playhead crosses
- [ ] C. A shortcut that splits the selected clips at the playhead (name the key in the notes)
- [ ] Other / my notes:

Your recommendation: A only, because it is what you asked for.

### Q91 — Where exactly does the cut land?
Context: See Q53 and Q54.

- [ ] A. At the mouse position, snapped to the grid when Snap is on, with the same bypass key as trim
- [ ] B. Always exactly at the mouse position
- [ ] Other / my notes:

Your recommendation: A, because cuts on the beat are the common case.

### Q92 — What shows where the cut will land?
Context: Drop phantoms are dashed lines in the sky accent colour (`theme.accent`).

- [ ] A. A vertical dashed accent line on the hovered clip only, at the (snapped) cut position
- [ ] B. A vertical dashed accent line across all tracks
- [ ] C. Nothing but the cursor
- [ ] Other / my notes:

Your recommendation: A, because it shows the snapped position, which the cursor alone cannot.

### Q93 — Which cursor in the Cut tool?
Context: The lasso already uses the standard crosshair.

- [ ] A. A standard cursor (`crosshair` over clips, the arrow elsewhere)
- [ ] B. A custom razor-blade cursor (a new image asset)
- [ ] Other / my notes:

Your recommendation: A, because the preview line already tells you where the cut is and no asset is needed; B would suit the tape-splicing theme if you want it.

### Q94 — What happens to the fades when a clip is cut?
Context: A clip has one fade-in (attack) and one fade-out (decay). Your test requirement is that the two halves together sound exactly like the uncut clip.

- [ ] A. The left half keeps the fade-in and gets no fade-out; the right half keeps the fade-out and gets no fade-in
- [ ] B. Both halves copy both fades
- [ ] Other / my notes:

Your recommendation: A, because B puts a fade-out and fade-in at the cut, which you would hear as a dip and which fails the "halves equal the whole" test.

### Q95 — A fade is longer than the half it lands on. What then?
Context: For example a 2-second fade-in on a clip cut 1 second from its start. A clip cannot hold "the second half of a fade" today.

- [ ] A. The fade is shortened to its half's length. In this one case the pair sounds slightly different from the uncut clip
- [ ] B. The fade is split exactly across both halves (clips gain a "fade starts partway" property)
- [ ] C. A cut inside a fade is refused with a message
- [ ] Other / my notes:

Your recommendation: A, because the case is rare and B adds a permanent new clip property for it. The "halves equal the whole" test would then cover cuts outside the fades.

### Q96 — Confirm: what else each half inherits
Context: Clips have no name or colour of their own; the label is the file name (`PlaylistObject.ts:112-116`).

- [ ] Confirm: both halves keep the original's track, gain, pan, pitch and stretch, and both show the same file name. No name or colour property is added
- [ ] Other / my notes:

Your recommendation: Confirm, because anything else would change the sound.

### Q97 — Which half is "the original" after a cut?
Context: This decides what stays selected and what the Sample tab keeps showing.

- [ ] A. The left half is the original clip (it stays selected and inspected if it was); the right half is a new, unselected clip
- [ ] B. Both halves are selected afterwards
- [ ] C. Neither is selected afterwards
- [ ] Other / my notes:

Your recommendation: A, because it changes the least.

### Q98 — Can one click cut several clips?
Context: The brief asks about multi-clip and multi-track cuts.

- [ ] A. No: a click cuts only the clicked clip
- [ ] B. A click on a selected clip also cuts every other selected clip that crosses that position
- [ ] C. Dragging a vertical line across tracks cuts every clip the line crosses
- [ ] Other / my notes:

Your recommendation: B, because it reuses the selection you already have and adds no new gesture.

### Q99 — The cut position is closer to an edge than the minimum clip length. What happens?
Context: See Q55.

- [ ] A. Nothing: no cut is made and the preview line is not shown there
- [ ] B. The cut is moved inward to the nearest allowed position
- [ ] Other / my notes:

Your recommendation: A, because a cut that lands somewhere you did not click is worse than no cut.

### Q100 — Does the cut happen on press or on release?
Context: —

- [ ] A. On release, if the mouse has not moved more than the drag threshold. Moving away first cancels
- [ ] B. On press
- [ ] Other / my notes:

Your recommendation: A, because it gives you a way to back out, and it keeps "drag on empty space pans" working in the Cut tool.

### Q101 — Confirm: cutting a stretched clip
Context: Skip if Q72 is B.

- [ ] Confirm: both halves keep the same stretch, and the two halves together still play exactly what the uncut stretched clip played
- [ ] Other / my notes:

Your recommendation: Confirm, because the numeric "halves equal the whole" test should hold for stretched clips too.

---

## Part 7 — Deep copy

### Q102 — When is "Deep copy" offered?
Context: You described it for the case where two or more clips share a source.

- [ ] A. On every sample clip, always
- [ ] B. Only when another clip uses the same file, or the clip is trimmed, stretched or has changed settings
- [ ] Other / my notes:

Your recommendation: A, because a rule about when the item appears is one more thing to explain, and baking a lone clip is harmless.

### Q103 — Does the deep copy replace the clip or add a new one?
Context: FL Studio's "Make unique as sample" replaces the clip in place.

- [ ] A. Replace in place: the same clip, at the same position and track, now plays the new file
- [ ] B. Add: the original stays and a new clip playing the new file is placed beside it
- [ ] Other / my notes:

Your recommendation: A, because it is what the FL feature you named does.

### Q104 — What is baked into the file? (tick all that apply)
Context: A clip's sound passes through trim, pitch, stretch, fades and pan (all per clip), then the track's mixer strip, then the master. Gain is asked separately in Q105. Whatever is baked is reset to neutral on the clip afterwards, so it is not applied twice.

- [ ] A. Trim
- [ ] B. Pitch (and the speed change that comes with it)
- [ ] C. Stretch
- [ ] D. Fades (attack and decay)
- [ ] E. Clip pan
- [ ] F. The track's mixer strip (volume, pan, mute)
- [ ] G. The master volume and limiter
- [ ] Other / my notes:

Your recommendation: A to E, because they are the clip's own properties. F and G belong to the track and the mix, and baking them would apply them twice on playback.

### Q105 — What about clip gain?
Context: Clip gain goes up to +12 dB, so a baked file can contain samples above full scale. Playback currently converts every file to 16 bits, which would clip those samples, and the copy would then not sound like the original.

- [ ] A. Gain is not baked: the new file is written at unity gain and the clip keeps its gain setting. Playback is guaranteed to match
- [ ] B. Gain is baked, and I change how files are loaded so that float files play back without clipping (this brings the "playback is 16-bit" item from `later.md` into scope)
- [ ] C. Gain is baked and anything above full scale is clipped in the file
- [ ] Other / my notes:

Your recommendation: A, because it is the only option that keeps "sounds identical" true with today's playback and without a rework. The file then holds everything except loudness.

### Q106 — How close must the copy be to the original?
Context: Playback converts files to 16 bits. The original clip is processed in floating point and never re-quantised; the copy is processed, written, then converted to 16 bits when played. So bit-identical is not possible with today's playback.

- [ ] A. Same number of frames, and every sample within one 16-bit step (about 0.00003 of full scale, −90 dBFS), scaled by the clip's gain
- [ ] B. Tighter than that, which requires replacing the 16-bit playback path first
- [ ] Other / my notes:

Your recommendation: A, because −90 dBFS is far below audibility and B is a separate engine project.

### Q107 — What sample rate is the new file?
Context: The engine plays at the output device's rate. Playing a file at another rate resamples it with plain linear interpolation.

- [ ] A. The engine's current output rate, so the file plays back with no resampling. If no output device is open, the source file's rate
- [ ] B. Always the source file's rate
- [ ] C. Always 48 000 Hz
- [ ] Other / my notes:

Your recommendation: A, because it is the only choice that reproduces "as it currently plays" exactly on your device.

### Q108 — What format and bit depth?
Context: Recorded takes are 32-bit float WAV (`input.rs:247-252`).

- [ ] A. WAV, 32-bit float
- [ ] B. WAV, 24-bit
- [ ] C. WAV, 16-bit (what playback uses today; smallest files)
- [ ] Other / my notes:

Your recommendation: A, because it matches the app's own recordings and loses nothing if playback is improved later.

### Q109 — Mono or stereo?
Context: A clip's processed sound is always stereo, because pan is applied even at centre.

- [ ] A. Always stereo
- [ ] B. Mono when the source is mono and pan is centred; stereo otherwise
- [ ] Other / my notes:

Your recommendation: A, because it is exactly what plays and has no special cases. Mono sources produce files twice as large as they need to be.

### Q110 — How are the files named?
Context: Clip labels come from the file name. Takes are named `<project>_<MMDDYYYY>_<n>.wav`.

- [ ] A. After the source: `<source name>_copy_<n>.wav`, for example `kick_copy_1.wav`
- [ ] B. Like takes: `<project>_<MMDDYYYY>_<n>.wav`
- [ ] C. You are asked for a name each time
- [ ] Other / my notes:

Your recommendation: A, because the clip's label then still tells you what it is.

### Q111 — Confirm: a deep copy never overwrites a file
Context: The helpers that name takes pick the next free number (`project_files.rs:172-196`).

- [ ] Confirm: the next free number is always used, names are compared without regard to upper and lower case, and no existing file is ever overwritten
- [ ] Other / my notes:

Your recommendation: Confirm, because overwriting audio is irreversible.

### Q112 — What happens to the old source file when nothing uses it any more?
Context: Sources can be library samples outside the project or takes in `recordings/`.

- [ ] A. Nothing, ever. Deep copy never deletes or moves any file
- [ ] B. If the old file is inside the project folder and no clip uses it, you are asked whether to move it to the Trash
- [ ] Other / my notes:

Your recommendation: A, because a file might be used by another project, and deleting is not something a "copy" command should do.

### Q113 — Deep copy in a project that has never been saved
Context: An unsaved project has no folder. Recordings in that situation go to the app's data folder and move into the project on first save. Your own failure criterion says a deep copy must never write outside the project folder.

- [ ] A. The Save dialog opens first; the deep copy happens after the project is saved, and is abandoned if you cancel
- [ ] B. It is refused with a message asking you to save the project first
- [ ] C. Like recordings: written to the app's data folder and moved into `clips/` on first save
- [ ] Other / my notes:

Your recommendation: A, because it honours your failure criterion and does not make you repeat the command.

### Q114 — Confirm: older projects that are a bare file
Context: Projects saved before folders existed are a single `.dawesome` file with `recordings/` beside it.

- [ ] Confirm: for those, deep copies go in a `clips/` folder beside the project file, the same way `recordings/` does
- [ ] Other / my notes:

Your recommendation: Confirm, because it mirrors the existing rule.

### Q115 — What does Save As do with `clips/`?
Context: Save As copies `recordings/` into the new project folder so the old project keeps its own (`project.rs:511-519`).

- [ ] A. The same: `clips/` is copied into the new project folder, and the new project's clips point at the copies
- [ ] B. `clips/` is not copied; the new project points at the old project's files
- [ ] Other / my notes:

Your recommendation: A, because a project folder should be self-contained, as it already is for recordings.

### Q116 — Do deep copies appear in a sidebar browser?
Context: Recordings have their own sidebar page.

- [ ] A. No; this project adds no browser for `clips/`
- [ ] B. They appear in the Recordings page
- [ ] C. A new "Clips" sidebar page
- [ ] Other / my notes:

Your recommendation: A, because it is not in the goals.

### Q117 — What do you see while a deep copy is being written?
Context: An offline bounce runs much faster than real time, so most clips finish in well under a second.

- [ ] A. Nothing for fast ones; if it takes longer than about half a second, the mouse shows the wait cursor until it finishes
- [ ] B. A progress bar
- [ ] C. Nothing at all
- [ ] Other / my notes:

Your recommendation: A, because a progress bar for a sub-second job is noise.

### Q118 — Confirm: what a failed deep copy does
Context: `ProjectNotice` is the app's banner for errors (`src/components/ProjectNotice.tsx`).

- [ ] Confirm: on any failure (disk full, no permission) the clip is left exactly as it was, any partly written file is removed, and the reason is shown in the existing notice banner
- [ ] Other / my notes:

Your recommendation: Confirm, because a half-finished copy must never replace working audio.

### Q119 — Undoing a deep copy: what happens to the file?
Context: Skip if Q10 is A.

- [ ] A. Undo puts the clip back on its original file with its original trim and settings. The new file stays on disk. Redo reuses that file and does not write another
- [ ] B. Undo also moves the new file to the Trash
- [ ] C. Deep copy cannot be undone
- [ ] Other / my notes:

Your recommendation: A, because undo should never delete audio. Unused files can accumulate in `clips/`; a clean-up command would be a later feature.

### Q120 — Is the project saved automatically after a deep copy?
Context: The new file exists on disk at once, but the project file only mentions it after a save.

- [ ] A. No. If you quit without saving, the new file is left unused in `clips/`
- [ ] B. Yes, the project is saved straight after
- [ ] Other / my notes:

Your recommendation: A, because nothing else in the app saves on your behalf.

### Q121 — Confirm: the copy is exactly as long as the clip
Context: —

- [ ] Confirm: the new file holds exactly the clip's played length, with no added silence or tail
- [ ] Other / my notes:

Your recommendation: Confirm, because the clip must stay the same size on the timeline.

### Q122 — Confirm: the menu label
Context: —

- [ ] Confirm: the item is labelled exactly "Deep copy" and has no keyboard shortcut
- [ ] Other / my notes:

Your recommendation: Confirm, because that is the name you chose.

---

## Part 8 — Clipboard and Repeat

### Q123 — Confirm: what Copy puts on the clipboard
Context: —

- [ ] Confirm: Cmd+C copies every selected clip (subject to Q26 for pattern clips) with its track, its position relative to the others, its trim, its stretch and its settings. With nothing selected, Cmd+C and Cmd+X do nothing
- [ ] Other / my notes:

Your recommendation: Confirm, because a paste should reproduce what you copied.

### Q124 — Where in time does Paste put the clips?
Context: The playhead is where playback starts from; the Stop key returns it to the start, and clicking the ruler moves it.

- [ ] A. At the playhead: the earliest copied clip starts there and the rest keep their spacing
- [ ] B. At the mouse position if the mouse is over the Playlist, otherwise at the playhead
- [ ] C. At the original position (paste in place)
- [ ] Other / my notes:

Your recommendation: A, because it is visible and deliberate. Since Stop rewinds, you would click the ruler to choose the spot.

### Q125 — Paste while the playlist is playing
Context: Skip unless Q124 is A or B. While playing, the playhead is moving and is rarely on a grid line.

- [ ] A. The paste lands at the playhead's position at the moment of the key press, snapped to the grid when Snap is on
- [ ] B. The paste lands where playback last started from
- [ ] C. Paste does nothing while playing
- [ ] Other / my notes:

Your recommendation: A, because it is the same rule as when stopped, with the snap you would expect.

### Q126 — Which tracks do pasted clips go on?
Context: —

- [ ] A. The same tracks they were copied from
- [ ] B. Starting at the track under the mouse, keeping their vertical spacing
- [ ] C. Starting at the track of the currently selected clip
- [ ] Other / my notes:

Your recommendation: A, because it needs no pointing and is always predictable.

### Q127 — What if the paste lands on top of existing clips?
Context: Clips can overlap on a track today, and both play.

- [ ] A. It overlaps them, as a move would
- [ ] B. Pasted clips go to the nearest free tracks instead
- [ ] C. The paste is refused
- [ ] Other / my notes:

Your recommendation: A, because it is the existing rule.

### Q128 — What if part of the paste would start past the end of the playlist?
Context: Clip starts are clamped into the playlist (see Q7). This rule also applies to Repeat.

- [ ] A. Nothing is pasted, and a short message says why
- [ ] B. The clips that fit are pasted; the rest are dropped
- [ ] C. Clips past the end are pulled back to the last beat
- [ ] Other / my notes:

Your recommendation: A, because a partial or squashed paste silently breaks the arrangement.

### Q129 — You press Cmd+V twice. Where does the second paste go?
Context: Skip unless Q124 is A or B.

- [ ] A. The same place as the first, on top of it
- [ ] B. Straight after the first: when stopped, each paste moves the playhead to the end of what was pasted
- [ ] Other / my notes:

Your recommendation: A, because Repeat already exists for "again, after this", and A has no hidden side effect on where playback starts.

### Q130 — What is selected after a paste?
Context: —

- [ ] A. The pasted clips become the selection
- [ ] B. The selection does not change
- [ ] Other / my notes:

Your recommendation: A, because you usually move or repeat what you just pasted.

### Q131 — Does Cmd+X leave a gap?
Context: —

- [ ] A. Yes: the clips are removed and nothing else moves
- [ ] B. No: later clips on those tracks slide left to close the gap
- [ ] Other / my notes:

Your recommendation: A, because nothing else in the Playlist moves clips you did not touch.

### Q132 — Confirm: pasted clips share audio but are independent clips
Context: —

- [ ] Confirm: a pasted sample clip plays the same file as the clip it was copied from (no new file is written) and has its own independent trim, stretch and settings. A pasted pattern clip is linked to the same pattern
- [ ] Other / my notes:

Your recommendation: Confirm, because that is the "shared source" behaviour Deep copy exists to break.

### Q133 — Does the clipboard survive opening another project?
Context: Pattern clips refer to patterns that exist only in their own project.

- [ ] A. No: New Project and Open empty the clipboard
- [ ] B. Yes: sample clips can be pasted into another project; pattern clips are skipped
- [ ] Other / my notes:

Your recommendation: A, because it avoids pasting clips whose patterns do not exist.

### Q134 — Confirm: the clip clipboard is private to Dawesome
Context: —

- [ ] Confirm: copying clips does not change the system clipboard, and text you copied elsewhere is still there afterwards. In a text field, Cmd+C, Cmd+V and Cmd+X act on the text and never on clips
- [ ] Other / my notes:

Your recommendation: Confirm, because clips mean nothing to other apps.

### Q135 — Should the app get an Edit menu?
Context: The app's menu bar has no Edit menu (`engine/src/app/window.rs:102-107`). On macOS, copy and paste in text fields normally depend on Edit-menu items; I have not tested whether they work in the app's text fields today.

- [ ] A. Yes: an Edit menu with Undo and Redo (if built), Cut, Copy, Paste and Repeat, showing their shortcuts. The items act on clips when the Playlist is focused and on text while typing
- [ ] B. No menu; the shortcuts are handled inside the window only
- [ ] Other / my notes:

Your recommendation: A, because it is the macOS convention, it makes the shortcuts discoverable, and it is the reliable way to keep text copy and paste working.

### Q136 — Should Cmd+A select every clip?
Context: Not in your goals; it is a natural partner to Copy.

- [ ] A. No
- [ ] B. Yes, when the Playlist is focused and you are not typing
- [ ] Other / my notes:

Your recommendation: A, because the brief says not to add features. It is cheap if you want it.

### Q137 — Repeat: where does the duplicate start?
Context: You wrote "paste a duplicate of the selection immediately after itself". The selection may hold several clips, gaps and several tracks. Its span runs from the earliest start to the latest end.

- [ ] A. Exactly at the span's end, so the copy touches the original with no gap
- [ ] B. At the span's end, rounded up to the next snap step when Snap is on; exactly at the end when Snap is off
- [ ] C. At the span's end rounded up to the next bar, always
- [ ] Other / my notes:

Your recommendation: B, because a 3.7-beat sample then repeats every 4 beats and stays on the grid, which is what you want when building a loop.

### Q138 — What is selected after a Repeat?
Context: —

- [ ] A. The new duplicates, so pressing Cmd+R again continues the chain
- [ ] B. The originals stay selected
- [ ] Other / my notes:

Your recommendation: A, because repeated presses then lay copies end to end.

### Q139 — Confirm: Repeat's remaining rules
Context: —

- [ ] Confirm: duplicates go on the same tracks as their originals and keep the gaps between them; with nothing selected Cmd+R does nothing; the end-of-playlist rule is the one from Q128; the duplicates share audio like pasted clips (Q132); Repeat does not change the clipboard
- [ ] Other / my notes:

Your recommendation: Confirm, because it should behave like copy and paste done for you.

### Q140 — Confirm: Cmd+R never reloads the window
Context: Cmd+R is the browser's reload key. Nothing blocks it today; whether the app's webview reloads on it is untested.

- [ ] Confirm: Cmd+R never reloads the app window, in development or release builds, whichever view is focused
- [ ] Other / my notes:

Your recommendation: Confirm, because a reload would throw away the frontend's state mid-session.

---

## Part 9 — Join

### Q141 — What does Join do to sample clips?
Context: You wrote that two selected things "become one". Two unrelated samples can only become one clip if a new audio file is made. Two halves of a cut can become one clip again without a new file.

- [ ] A. Bounce: the selected clips are rendered together into one new file in `clips/` and replaced by one clip
- [ ] B. Heal only: Join works only on neighbouring pieces of the same file that still line up (it undoes a cut) and writes no file
- [ ] C. Both: heal when the pieces line up, bounce otherwise
- [ ] D. Group: the clips stay separate but move and select as one
- [ ] Other / my notes:

Your recommendation: C, because it does the cheap, lossless thing when it can and still always produces one clip.

### Q142 — Can clips on different tracks be joined?
Context: Each track has its own mixer strip, which is not part of a clip.

- [ ] A. No: Join needs all selected clips on one track, and says so otherwise
- [ ] B. Yes: they are mixed into one clip on the topmost of their tracks
- [ ] Other / my notes:

Your recommendation: A, because moving audio between tracks changes the mix (different volume, pan and mute).

### Q143 — Confirm: gaps and overlaps in a bounced join
Context: Skip if Q141 is B or D.

- [ ] Confirm: the new clip runs from the earliest start to the latest end; gaps between the clips are silence in the file; overlapping clips are summed, as they play today
- [ ] Other / my notes:

Your recommendation: Confirm, because the joined clip must sound like the clips it replaced.

### Q144 — Confirm: a bounced join follows the Deep copy answers
Context: Skip if Q141 is B or D.

- [ ] Confirm: format, sample rate, what is baked, gain handling, the unsaved-project rule, error handling and undo are all whatever you chose for Deep copy
- [ ] Other / my notes:

Your recommendation: Confirm, because it is the same operation on several clips.

### Q145 — What does Join do to pattern clips?
Context: Skip if you did not tick F in Q26. Patterns can only be 1, 2 or 4 bars long today (`BAR_CHOICES`, `sequencer.rs:18`).

- [ ] A. It makes a new pattern containing the selected clips' steps in sequence and replaces the clips with one clip of it. The original patterns are untouched
- [ ] B. It bounces the pattern clips to audio, like sample clips
- [ ] Other / my notes:

Your recommendation: A, because the result stays an editable pattern. The 1, 2 or 4 bar limit brings follow-up questions in the next batch.

### Q146 — The selection holds both sample clips and pattern clips. What does Join do?
Context: —

- [ ] A. Nothing, with a short message that Join needs clips of one kind
- [ ] B. Everything is bounced to one audio clip
- [ ] C. The sample clips are joined with each other and the pattern clips with each other
- [ ] Other / my notes:

Your recommendation: A, because the other two results are hard to predict from looking at the selection.

### Q147 — How is a bounced join's file named?
Context: Skip if Q141 is B or D. See Q110.

- [ ] A. After the first clip's source: `<source name>_join_<n>.wav`
- [ ] B. The same scheme you chose for deep copies
- [ ] Other / my notes:

Your recommendation: A, because the label then tells you the clip is a join.

---

## Part 10 — Context menu

### Q148 — Which items does a sample clip's menu have? (tick all that apply)
Context: Today it has only Delete. You asked for "Deep copy" and said Repeat could be there too.

- [ ] A. Cut (⌘X)
- [ ] B. Copy (⌘C)
- [ ] C. Paste (⌘V)
- [ ] D. Repeat (⌘R)
- [ ] E. Join (⇧J), shown when two or more clips are selected
- [ ] F. Deep copy
- [ ] G. Delete (⌫), as now
- [ ] Other / my notes:

Your recommendation: A, B, D, E, F and G, because each acts on the clip you clicked. Paste is left out because it acts on a position, not a clip.

### Q149 — Confirm: the menu's order
Context: The menu has no divider lines today; this adds them.

- [ ] Confirm: Cut, Copy, Repeat; a divider; Join, Deep copy (and Reset stretch, if Q86 is A); a divider; Delete. A pattern clip keeps "Explode into tracks" and "Open in Sequencer" at the top, as now
- [ ] Other / my notes:

Your recommendation: Confirm, because it groups clipboard actions, then actions that change audio, then the destructive one last.

### Q150 — You right-click a clip that is part of a selection. What do menu actions apply to?
Context: Today Delete in the menu removes only the right-clicked clip, even when others are selected (`PlaylistCanvas.tsx:69-75`).

- [ ] A. The whole selection (Delete included, which changes its current behaviour)
- [ ] B. Only the right-clicked clip, for every item
- [ ] Other / my notes:

Your recommendation: A, because Cut, Copy, Repeat and Join only make sense on the selection, and a menu where Delete alone ignores the selection would be a trap.

### Q151 — You right-click a clip that is not selected. What happens to the selection?
Context: Right-clicking does not change the selection today.

- [ ] A. That clip becomes the selection, and the menu acts on it
- [ ] B. The selection is unchanged, and the menu acts on the right-clicked clip only
- [ ] Other / my notes:

Your recommendation: A, because you can then see what the menu will act on.

### Q152 — Does right-clicking empty space open a menu?
Context: It does nothing today.

- [ ] A. No
- [ ] B. Yes, with Paste, which pastes at that position and track
- [ ] Other / my notes:

Your recommendation: A, because it is not in the goals and it would add a second paste rule.

### Q153 — Confirm: the menu keeps its current look
Context: `ClipMenu` is a plain white popover, not styled like the four-track controls.

- [ ] Confirm: the menu keeps its current look; this project only adds items and divider lines
- [ ] Other / my notes:

Your recommendation: Confirm, because you said styles are not being reworked.

---

## Part 11 — Space

### Q154 — What exactly does Space do?
Context: There are three behaviours in the app today. The topbar Play/Pause key toggles between play and pause, keeping the playhead when paused, and in the Sequencer view it loops the selected pattern instead (`togglePlayHere`, `sequencerSlice.ts:248-265`). The Stop key stops and returns to the start.

- [ ] A. The same as the topbar Play/Pause key, including its Sequencer behaviour
- [ ] B. Play, and when playing, stop and return to the start (like the Stop key)
- [ ] C. Play, and when playing, stop and return to where playback last started
- [ ] Other / my notes:

Your recommendation: A, because it is one existing behaviour reused, so the key and the button can never disagree. C is a new transport concept.

### Q155 — Where does Space work?
Context: —

- [ ] A. Everywhere in the app, whichever view is focused, except while typing
- [ ] B. Only when the Playlist is the focused view
- [ ] Other / my notes:

Your recommendation: A, because transport keys are global in every DAW.

### Q156 — Confirm: Space never triggers transport while typing
Context: "Typing" is defined by `isTypingTarget` (`src/util/keys.ts:3-14`): text fields, the tempo field, rename fields, search, drop-down lists and editable text.

- [ ] Confirm: in any of those, Space types a space (or does the field's own thing) and never starts or stops playback
- [ ] Other / my notes:

Your recommendation: Confirm, because it is your own failure criterion.

### Q157 — What about controls that already use Space when focused?
Context: A focused sequencer step toggles on Space, a focused sidebar sample row previews, a focused wind key winds, and any focused button is "clicked" by Space. After you click Stop with the mouse, the Stop button keeps focus.

- [ ] A. Space always means transport, outside text fields. Those controls keep Enter and lose Space
- [ ] B. The focused control wins: Space does that control's action, and is transport only when nothing with a Space action is focused
- [ ] C. Like A, but focused sidebar sample rows keep Space for preview
- [ ] Other / my notes:

Your recommendation: A, because with B, Space after clicking Stop would press Stop again, and you could never predict what the key does.

### Q158 — Confirm: holding Space does not repeat
Context: —

- [ ] Confirm: holding Space down toggles once, not repeatedly
- [ ] Other / my notes:

Your recommendation: Confirm, because repeated toggling would stutter.

### Q159 — Confirm: Space while recording
Context: Ending playback already ends a recording and places its takes (`end_run`, `daw.rs:367-389`).

- [ ] Confirm: Space during a recording stops playback and finishes the take, exactly as the Play/Pause key does now
- [ ] Other / my notes:

Your recommendation: Confirm, because Space should not have recording rules of its own.

---

## Part 12 — Shortcuts and cross-cutting

### Q160 — Cmd on macOS, Ctrl elsewhere?
Context: The native menu uses "Cmd or Ctrl" accelerators. Code inside the window treats the two keys alike today.

- [ ] A. Cmd on macOS only; Ctrl on Windows and Linux only
- [ ] B. Either key works on every platform
- [ ] Other / my notes:

Your recommendation: A, because on macOS Ctrl+C and Ctrl+V are not copy and paste anywhere else.

### Q161 — Are letter shortcuts matched by character or by key position?
Context: The existing Shift+E handler matches the character (`isExplodeKey`, `src/util/keys.ts:22-25`).

- [ ] A. By character: the key that types C on your layout
- [ ] B. By physical position: the key where C sits on a US keyboard
- [ ] Other / my notes:

Your recommendation: A, because it is the existing convention and matches what is printed on the key.

### Q162 — Approve the full shortcut table
Context: Existing shortcuts are unchanged unless marked. "Playlist" scope means the Playlist is the focused view and you are not typing. Rows that depend on another answer say so.

| Keys | Action | Scope | Status | Conflicts and notes |
|---|---|---|---|---|
| Space | Play / stop (Q154) | Everywhere, not typing | New | Takes Space from focused buttons, step cells, wind keys, sample rows (Q157). Cmd+Space and Ctrl+Space belong to macOS and are ignored |
| S | Select tool | Playlist | New | Cmd+S stays Save |
| C | Cut tool | Playlist | New | Cmd+C is Copy |
| Escape | Clear selection; back to Select tool (Q36) | As today | Changed | Still also closes menus and cancels drags |
| Cmd+C | Copy clips | Playlist | New | Copies text in text fields |
| Cmd+X | Cut clips | Playlist | New | Cuts text in text fields |
| Cmd+V | Paste clips | Playlist | New | Pastes text in text fields |
| Cmd+R | Repeat | Playlist | New | Webview reload is blocked everywhere (Q140) |
| Shift+J | Join | Playlist | New | None found |
| Cmd+Z | Undo (if Q10 is B or C) | Q19 | New | Undoes typing in text fields |
| Shift+Cmd+Z | Redo (if Q10 is B or C) | Q19 | New | — |
| Option held during a drag | Bypass snap (if Q54 is B or C) | Playlist drags | New | — |
| Shift+drag a clip edge | Stretch | Playlist | New | Shares Shift with selection (Q81) |
| Shift+drag over a clip | Lasso (Q39) | Playlist | Changed | — |
| Shift+click a clip | Toggle in selection | Playlist | Existing | — |
| Shift+E | Explode selected pattern clips | Playlist | Existing | Cmd+Shift+E opens the Sequencer tab |
| Delete / Backspace | Delete selected clips | Playlist | Existing | — |
| Cmd+N, Cmd+O, Cmd+S, Shift+Cmd+S | New, Open, Save, Save As | Menu | Existing | — |
| Shift+Cmd+M, Shift+Cmd+E | Open Mixer, open Sequencer | Menu | Existing | — |
| Cmd + wheel, pinch | Zoom | Playlist canvas | Existing | — |

- [ ] Confirm: the table is approved as listed
- [ ] Other / my notes:

Your recommendation: Confirm, because I found no remaining collision once the rows above are handled. One thing I noticed and am not changing: Quit has no Cmd+Q shortcut in the menu.

### Q163 — What performance must the Playlist hold?
Context: The canvas draws at up to 60 frames per second and has a built-in frame-rate readout in its debug overlay. I do not know your typical project size.

- [ ] A. 200 clips in the project; edge drags, cut previews and lassos stay at 50 frames per second or better on your Mac; every edit command returns within 50 ms
- [ ] B. 500 clips, same frame rate and response time
- [ ] C. 1000 clips, same frame rate and response time
- [ ] Other / my notes:

Your recommendation: B, because it is well beyond what a 16-beat playlist holds and still realistic to verify with a generated test project.

### Q164 — How fast must a deep copy be?
Context: A bounce is an offline render, far faster than real time.

- [ ] A. A one-minute clip is written in under 2 seconds on your Mac
- [ ] B. No stated target
- [ ] Other / my notes:

Your recommendation: A, because a number makes it testable. If Q72 is A, a stretched clip's first render is measured separately.

### Q165 — Confirm: the look and feel rule
Context: Canvas colours live in `sketch/theme.ts`; controls live in `src/components/fourtrack/`.

- [ ] Confirm: no existing style changes. New canvas marks (cut line, live trim, stretch label) use colours already in `theme.ts`. New controls are built from existing four-track components. New cursors are standard system cursors unless you chose a custom one above
- [ ] Other / my notes:

Your recommendation: Confirm, because you said this project follows the established style and does not rework it.
