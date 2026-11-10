# 01 — Answers to design questions, batch 1

You asked to answer in chat as a questionnaire instead of ticking boxes in `01-design-questions.md`. This file records each answer as you gave it, in the same numbering. Text in quotes is your own wording. Follow-up questions asked on the spot are numbered with a letter (for example Q7a).

## Part 0 — How we work

| Q | Answer |
|---|---|
| Q1 Who commits | Your words: "I'll allow you to commit for just this one project. Try to copy my commit writing style - minimal". I commit at the end of each milestone, this project only, with short messages in the style of your existing ones. |
| Q2 Branch | A. All work on `playlist-enhancements`. |
| Q3 `docs/todo.md` | Confirmed. I never edit, stage or commit it. |
| Q4 Platforms | A. macOS only. |
| Q5 Milestone gate | A. Both test suites pass and my changes add no new eslint or tsc problems. |
| Q6 Gesture testing | A. Unit tests on extracted logic, numeric engine tests, and a manual checklist you run each milestone. |

## Part 1 — Foundations

| Q | Answer |
|---|---|
| Q7 Playlist length | B. A longer fixed length. |
| Q7a How many bars | 64 bars. |
| Q8 Tempo | A. Fix as groundwork: clips keep their beat position when the tempo changes. |
| Q9 Trimmed length is audio, not beats | Confirmed. |
| Q10 Undo scope | C. Whole project (clips, clip settings, mixer, sequencer, tempo). |
| Q11 Core undoable list | Confirmed: adding, moving, deleting clips; clip settings; exploding a pattern clip; mixer changes (volume, pan, mute, solo, track names, reset); sequencer edits (rows, patterns, steps, bars, names, folders); tempo; every new operation in this project. |
| Q11b Borderline items | None of these: clips placed by a recording, project toggles (metronome, loop, snap, count-in) and track arming are not undo steps. |
| Q12 Where history lives | A. In the engine. |
| Q13 Shortcuts | A. Cmd+Z undo, Shift+Cmd+Z redo. |
| Q14 One gesture = one step | Confirmed, including the whole-project rows: one knob or fader drag = 1; one stroke painting sequencer steps = 1; one visit to the tempo field = 1; each mute, solo, rename, add or remove = 1. |
| Q15 Depth | A. 100 steps. |
| Q16 Lifetime | Confirmed: cleared by New and Open, survives Save, not written to the project file. |
| Q17 Not undo steps | Confirmed: selection, tool, zoom, scroll, playhead, play/stop, tab and pane layout. |
| Q18 Selection after undo/redo | A. The clips just changed or restored become the selection. |
| Q19 When Cmd+Z acts | A. Any view, whenever not typing. |
| Q20 No unsaved-changes indicator | Confirmed. |

## Part 2 — Data model

| Q | Answer |
|---|---|
| Q21 "Same source" = same file | Confirmed. |
| Q22 Trimming never changes the file | Confirmed. |
| Q23 Clips never affect each other | Confirmed. |
| Q24 File version | A. Version 3 only when a clip is trimmed or stretched. |
| Q25 Existing projects open unchanged | Confirmed. |
| Q26 Features on pattern clips | Ticked: copy/cut/paste/Repeat, trim by dragging edges, the Cut tool, Join, Deep copy. Not ticked: stretch. |
| Q27 Feature does not apply | A. Nothing happens; the menu item is left out. |

## Part 3 — Tools and selection

| Q | Answer |
|---|---|
| Q28 "Shift mode" | A. A held modifier, as now. |
| Q29 The "2" | A. Example count; Join works on two or more. |
| Q30 Tools | Confirmed: Select and Cut only. |
| Q31 Select tool keeps today's behaviour | Confirmed. |
| Q32 Tool switcher place | Your words: "Inside the playlist and maybe we can move the snap controls inside the playlist tray as well". |
| Q32a Where inside the Playlist | A strip above the ruler; the canvas starts below it. |
| Q32b Snap controls | Both the Snap key and the snap-value drop-down move from the topbar into the Playlist tray. |
| Q33 Key look | A. LatchKeys with icons (arrow, razor blade). |
| Q34 Tool remembered | A. No; always Select at launch and after opening a project. |
| Q35 C key | A. Sticky. |
| Q36 Escape in Cut tool | Your words: "It only clears the selection. And to be clear: I don't think the clip needs to be selected in order to be cut. The cut tool works on whatever clip is below the mouse cursor". The tool stays on Cut. |
| Q37 Other gestures in Cut tool | A. Pan, zoom, right-click menu and Shift-select still work; clips cannot be moved or trimmed. |
| Q38 C and S scope | Confirmed. |
| Q39 Lasso over a clip | A. Shift+drag starts a lasso anywhere; Shift+click still toggles. |
| Q40 Lasso threshold | A. 10 px. |
| Q41 Clip under the lasso's start | A. Selected. |
| Q42 Add or replace | A. Always adds. |
| Q43 Hit rule | Confirmed unchanged. |
| Q44 Plain click selects | B. No; keep today's behaviour (a plain click selects nothing). |
| Q45 Cmd+click | A. No. |
| Q46 Trackpad and touch | Confirmed unchanged. |

## Part 4 — Edge drag: trim

| Q | Answer |
|---|---|
| Q47 Grab zone | A. 6 screen pixels inside each edge. |
| Q48 Narrow clips | A. Zones shrink so the middle third always moves the clip. |
| Q49 Zone height | Confirmed: full clip height. |
| Q50 Cursor | A. `ew-resize`. |
| Q51 Hover handle | B. A 2 px bar in the sky accent colour on the hovered edge. |
| Q52 Visual while trimming | A. The clip resizes live; the engine is told on release. |
| Q53 Snap target | A. The same timeline grid as moves. |
| Q54 Snap bypass | B. Holding Option bypasses snap for trim, stretch and cut only. |
| Q55 Minimum length | A. 10 ms of played audio. |
| Q56 Source boundary | A. The edge stops at the source boundary. |
| Q57 Left edge does not slide audio | Confirmed. |
| Q58 Right edge past playlist end | A. Allowed, as far as the source allows. |
| Q59 Overlap with neighbour | A. Allowed. |
| Q60 Fades follow trimmed edges | A. Yes. |
| Q61 Fade longer than clip | A. The stored fade is shortened, as now. |
| Q62 Audio while dragging | A. None. |
| Q63 Edit while clip is sounding | A. Heard from the clip's next start. |
| Q64 Multi-selection trim | B. Every selected clip trims by the same amount, each stopping at its own limits. |
| Q65 Edge press and selection | Confirmed: same as pressing the clip's middle (with Q44, pressing an unselected clip's edge clears the selection). |
| Q66 Trimmed indicator | C. A small notch on each trimmed edge. |
| Q67 Sample tab | B. The whole file, with the played region highlighted. |
| Q68 No numeric fields | Confirmed. |
| Q69 Escape cancels an edge drag | Confirmed. |
| Q70 Release outside canvas | A. The trim is applied. |
| Q71 Sideways auto-scroll | A. No. |

## Part 5 — Stretch

| Q | Answer |
|---|---|
| Q72 Stretch kind | B. Tape-style: stretching changes speed and pitch together. |
| Q73–Q76, Q78 | Not asked: they applied only to pitch-preserving stretch. |
| Q77 Stretch writes Pitch | Confirmed. The limit is Pitch's ±24 semitones (a quarter to four times the unpitched length). |
| Q79 Opposite edge fixed | Confirmed. |
| Q80 Applies to current content | Confirmed. |
| Q81 Reading a Shift-press over a clip | Confirmed: edge zone + drag = stretch; elsewhere + drag = lasso; no drag = toggle. |
| Q82 Shift changed mid-drag | A. Nothing; fixed at the press. |
| Q83 Stretch snaps like trim | Confirmed. |
| Q84 Visual while stretching | A. The clip squeezes or expands live. |
| Q85 Stretched indicator | Amber "modified" dot only (it appears anyway because Pitch is no longer 0). |
| Q86 Removing a stretch | Confirmed: set Pitch back to 0 in the Sample tab, or undo. No menu item. |
| Q87 Sample tab Reset | A. The five settings only; trim is left alone. |
| Q88 Tempo change | Confirmed: a stretched clip keeps its length in seconds. |
| Q89 Multi-selection stretch | Every selected clip is stretched by the same ratio, each stopping at its own limit. |

## Part 6 — Cut tool

| Q | Answer |
|---|---|
| Q90 Ways to cut | Ticked: Cut tool click; context menu "Split at playhead". Not ticked: a shortcut. |
| Q91 Cut position | Your words: "Mouse, snapped, with the option of the cut snapping to the play head if it is between two snap intervals and the users cursor is closer to the play head than either of the two snap boundaries". |
| Q91a Playhead snap with Snap off or Option held | No playhead snap; the cut lands exactly at the mouse. |
| Q91b Playhead as a snap point for edge drags | Yes: the same playhead rule applies to cut, trim and stretch. |
| Q91c Playhead snap while playing | Only while stopped or paused. |
| Q92 Cut preview | A. A dashed accent line on the hovered clip only. |
| Q93 Cut cursor | B. A custom razor-blade cursor. |
| Q93a Who makes it | I draw it as an SVG in the existing icon style and show it to you for approval; crosshair as the fallback. |
| Q94 Fades on halves | A. Left half keeps the fade-in, right half keeps the fade-out; nothing at the cut. |
| Q95 Fade longer than its half | A. The fade is shortened to its half. |
| Q96 What halves inherit | Confirmed: track, gain, pan, pitch; same file name; no name or colour property. |
| Q97 Which half is the original | A. The left half; the right half is new and unselected. |
| Q98 Multi-clip cut | The clip under the cursor is always cut; if it is part of a selection, every other selected clip crossing that position is cut too. |
| Q99 Cut too near an edge | A. No cut, no preview line. |
| Q100 Press or release | A. On release, if the mouse has not moved more than 10 px. |
| Q101 | Not asked: it applied only to pitch-preserving stretch. Covered by Q96. |

## Part 7 — Deep copy

| Q | Answer |
|---|---|
| Q102 When offered | A. Always. |
| Q103 Replace or add | A. Replace in place. |
| Q104 What is baked | Trim, pitch and speed, fades, clip pan, and in your words "Gain and every other parameter". Not the track mixer strip, not the master. |
| Q105 Gain | C. Baked, and anything above full scale is clipped in the file. |
| Q106 Tolerance | A. Same number of frames, every sample within one 16-bit step, except samples the original pushes above full scale, which the copy clips. |
| Q107 Sample rate | A. The engine's current output rate; the source file's rate if no output device is open. |
| Q108 Format | A. WAV, 32-bit float. |
| Q109 Channels | A. Always stereo. |
| Q110 Naming | A. `<source name>_copy_<n>.wav`. |
| Q111 Never overwrites | Confirmed. |
| Q112 Old source file | A. Nothing, ever. |
| Q113 Unsaved project | A. The Save dialog opens first; abandoned if you cancel. |
| Q114 Bare-file projects | Confirmed: `clips/` beside the project file. |
| Q115 Save As | A. `clips/` is copied, like recordings. |
| Q116 Sidebar browser | C. A new "Clips" sidebar page. |
| Q116a What the Clips page does | All four: list and preview; drag into the Playlist; rename and delete; folders and moving. |
| Q117 Progress | A. Wait cursor if it takes longer than about half a second. |
| Q118 Failure | Confirmed. |
| Q119 Undo | A. The clip goes back to its original file; the new file stays on disk; redo reuses it. |
| Q120 Autosave | A. No. |
| Q121 Exact length | Confirmed. |
| Q122 Label | Confirmed: "Deep copy", no shortcut. |

## Part 8 — Clipboard and Repeat

| Q | Answer |
|---|---|
| Q123 What Copy copies | Confirmed. |
| Q124 Paste position | A. At the playhead. |
| Q125 Paste while playing | A. At the playhead at the moment of the key press, snapped when Snap is on. |
| Q126 Tracks | A. The original tracks. |
| Q127 Occupied space | A. Overlaps. |
| Q128 Past the end | A. Nothing is pasted, with a message. |
| Q129 Cmd+V twice | A. The same place. |
| Q130 Selection after paste | A. The pasted clips. |
| Q131 Cmd+X gap | A. Leaves a gap. |
| Q132 Shared audio, independent clips | Confirmed. |
| Q133 Clipboard across projects | A. Emptied by New and Open. |
| Q134 Private clipboard | Confirmed. |
| Q135 Edit menu | A. Yes. |
| Q136 Cmd+A | B. Yes, selects every clip. |
| Q137 Repeat placement | A. Exactly at the span's end, no rounding. |
| Q138 Selection after Repeat | A. The new duplicates. |
| Q139 Repeat's remaining rules | Confirmed. |
| Q140 Cmd+R never reloads | Confirmed. |

## Part 9 — Join

| Q | Answer |
|---|---|
| Q141 Join on sample clips | C. Heal when the pieces line up, bounce otherwise. |
| Q142 Different tracks | A. No; one track only, with a message otherwise. |
| Q143 Gaps and overlaps | Confirmed: gaps are silence, overlaps are summed. |
| Q144 Follows Deep copy answers | Confirmed. |
| Q145 Join on pattern clips | A. A new combined pattern. |
| Q146 Mixed selection | A. Nothing, with a message. |
| Q147 Join file name | A. `<source name>_join_<n>.wav`. |

## Part 10 — Context menu

| Q | Answer |
|---|---|
| Q148 Items | Cut, Copy, Paste, Repeat, Join (two or more selected), Split at playhead, Deep copy, Delete. |
| Q149 Order | Confirmed: Cut, Copy, Paste, Repeat; divider; Split at playhead, Join, Deep copy; divider; Delete. Pattern clips keep Explode and Open in Sequencer at the top. |
| Q150 Right-click inside a selection | A. Actions apply to the whole selection, Delete included. |
| Q151 Right-click an unselected clip | The selection is unchanged and the menu acts on that clip only. |
| Q152 Empty-space menu | B. Yes, with Paste. |
| Q152a Paste from the empty-space menu | Confirmed: at the right-clicked position (snapped when Snap is on), topmost copied clip on the right-clicked track. |
| Q153 Menu look | Confirmed unchanged. |

## Part 11 — Space

| Q | Answer |
|---|---|
| Q154 What Space does | A. The same as the topbar Play/Pause key, including its Sequencer behaviour. |
| Q155 Where | A. Everywhere, except while typing. |
| Q156 Never while typing | Confirmed. |
| Q157 Focused controls | A. Space is always transport; those controls keep Enter. |
| Q158 No key repeat | Confirmed. |
| Q159 While recording | Confirmed. |

## Part 12 — Shortcuts and cross-cutting

| Q | Answer |
|---|---|
| Q160 Cmd / Ctrl | A. Cmd on macOS only, Ctrl elsewhere only. |
| Q161 Letter matching | A. By character. |
| Q162 Shortcut list | Approved, with Cmd+A added and Escape left as it is today. |
| Q163 Performance | B. 500 clips, 50 frames per second or better, edits return within 50 ms. |
| Q164 Deep copy speed | A. A one-minute clip in under 2 seconds. |
| Q165 Look and feel rule | Confirmed. |
