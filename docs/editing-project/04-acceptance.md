# 04 — Acceptance checklist

Written 2026-11-10. Every item is a concrete, observable behaviour. Each is ticked only when the named kind of check has passed in this repo.

**How each item is checked**

- **[E]** an engine test in `cargo test` (numeric where audio is involved).
- **[U]** a frontend unit test in `pnpm ui:test`.
- **[M]** you do it by hand in the running app. Items marked [M] also have their logic covered by [U] or [E] tests where the plan says so; the manual pass confirms the real gesture.

Defaults used in the examples unless stated: 120 bpm, 4/4, Snap on at one beat, a 48 kHz output device.

Milestone tags (M1–M14) say when an item becomes checkable.

**Status convention:** a ticked item names the test that proves it. `[M]` items stay unticked until you have done them by hand; where their logic has a unit test, the item says so. I could not see the running app during the overnight build (screen capture returned a black image), so no `[M]` item has been checked by me.

---

## A. Timeline groundwork (M1, M2)

- [x] A1 [E] A clip can be placed and moved so that it starts at bar 60. A move aimed past the end is clamped to the end of bar 64. — `event_dispatch::tests::test_move_a_group_of_nodes`, `test_the_loop_ends_with_the_last_clip`, `test_beat_to_offset`
- [ ] A2 [M] At launch the Playlist shows bars 1 to 4 across its width, as it does today. (Opening zoom covered by `grid.spec.ts`.)
- [ ] A3 [M] Zooming out all the way shows 64 bars. Beat lines disappear before they smear together; bar lines and non-overlapping bar numbers remain. (Logic covered by `grid.spec.ts`.)
- [x] A4 [E] With Loop on and one clip ending at beat 10, the loop length is 12 beats (3 bars). With no clips it is 16 beats. — `state::playlist::tests::test_loop_end_after`, `event_dispatch::tests::test_the_loop_ends_with_the_last_clip`
- [ ] A5 [M] With Loop on, a marker on the ruler sits at the loop's end and moves when the last clip is moved or trimmed. With Loop off it is not drawn.
- [x] A6 [E] During a recording the loop length does not change when clips are added. — `recording::tests::test_the_loop_stays_put_while_recording`
- [ ] A7 [M] After Open and after New Project the view is back on bars 1 to 4.
- [x] A8 [E] A clip at beat 8 at 120 bpm is still at beat 8 after the tempo is set to 90, and its first rendered frame is at the frame for beat 8 at 90 bpm. — `audiograph::tests::test_a_clip_on_beat_8_is_heard_on_beat_8_at_another_tempo` (rendered frame 235,200 at 90 bpm), `event_dispatch::tests::test_clips_and_the_playhead_keep_their_beat_when_the_tempo_changes`
- [ ] A9 [M] After changing the tempo, each clip sits on the same grid line as before and plays on that beat, without saving and reloading.
- [x] A10 [E] A trimmed 1.000-second clip is still 1.000 second long after a tempo change. — `audiograph::tests::test_a_trimmed_clip_is_as_long_at_any_tempo`
- [ ] A11 [M] Changing the tempo during playback: playback continues from the same beat at the new tempo. (Engine half covered by `event_dispatch::tests::test_playback_carries_on_from_its_beat_at_the_new_tempo`.)
- [ ] A12 [M] While recording, the tempo field cannot be changed. (Engine refusal covered by `recording::tests::test_the_tempo_is_locked_while_recording`.)

## B. Undo and redo (M3, extended by each later milestone)

- [x] B1 [E] For every undoable command: apply, undo → the project equals the project before; redo → it equals the project after. — clips: `history::tests::test_adding_a_clip`, `test_moving_a_clip`, `test_moving_a_selection_is_one_step`, `test_deleting_a_selection_is_one_step`, `test_a_gesture_is_one_step` (clip settings); mixer: `test_mixer_changes`, `test_a_fader_drag_is_one_step`, `test_undoing_a_solo_lets_the_other_tracks_sound_again`; tempo: `test_tempo_changes`; Sequencer: `sequencer_commands::tests::test_every_sequencer_edit_can_be_undone_and_redone`, `test_placing_a_pattern_is_one_step`, `test_exploding_is_one_step`, `test_a_removed_row_and_pattern_come_back_as_themselves`. Extended by each later milestone.
- [ ] B2 [M] Move a clip, Cmd+Z: it returns and is selected. Shift+Cmd+Z: it moves again.
- [ ] B3 [M] Delete five selected clips, Cmd+Z once: all five return.
- [ ] B4 [M] Drag a mixer fader from −6 to 0 in one motion, Cmd+Z once: it is back at −6.
- [ ] B5 [M] Paint eight Sequencer steps in one stroke, Cmd+Z once: all eight clear.
- [ ] B6 [M] Type a new tempo and leave the field, Cmd+Z once: the old tempo returns.
- [x] B7 [E] After 150 edits, exactly 100 can be undone. — `history::tests::test_the_last_hundred_steps_are_kept`
- [x] B8 [E] Undo three steps, make one edit: redo does nothing. — `history::tests::test_a_new_change_ends_redo`, `test_a_change_made_during_an_undo_ends_redo`
- [ ] B9 [M] Cmd+Z works with the Mixer or Sequencer focused.
- [ ] B10 [M] Cmd+Z while typing in a rename field undoes typing and changes nothing in the project.
- [x] B11 [E] The history is empty after New Project and after Open, and unchanged by Save. — `history::tests::test_the_history_starts_again_with_a_project`, `test_saving_keeps_the_history`
- [x] B12 [E] Undo is refused while a recording is in progress. — `recording::tests::test_no_undo_or_redo_while_recording`
- [ ] B13 [M] Toggling metronome, loop or snap, or arming a track, adds no undo step.
- [ ] B14 [M] The Edit menu shows Undo, Redo, Cut, Copy, Paste, Repeat, Select All, Join, with plain Undo and Redo labels.

## C. Clip windows and project file (M4)

- [x] C1 [E] A whole clip sounds as it did before this project: the new reader's output equals the old path's within one 16-bit step. — `clip_reader::tests::test_a_whole_clip_is_read_as_it_always_was` (identical, bit for bit, with no fades), `test_a_whole_clip_with_every_setting_is_within_a_16_bit_step`
- [x] C2 [E] A project with no trimmed clip is still written as version 1 or 2, and an older-format fixture loads with every clip whole. — `project::tests::test_the_version_follows_what_a_project_uses`, `test_an_older_file_loads_whole_and_is_left_alone`
- [x] C3 [E] A project with a trimmed clip is written as version 3 and reloads with the same window. — `project::tests::test_a_trimmed_clip_is_saved_and_loaded`, `test_the_two_halves_of_a_cut_clip_come_back_to_the_nanosecond`, `test_a_pattern_clips_window_is_kept_past_the_patterns_end`
- [x] C4 [E] Changing one clip's window leaves the rendered output of another clip of the same file unchanged, sample for sample. — `audiograph::tests::test_changing_a_clip_changes_no_other_clip_of_the_sample`

## D. Space (M5)

Key handling is covered by `src/util/keys.spec.ts` ("the transport key"): Space toggles once per press, is kept from focused buttons on the way down and up, and is left alone in every kind of text field.

- [ ] D1 [M] Space starts playback; Space again pauses, and the playhead stays where it stopped.
- [ ] D2 [M] With the Sequencer focused and a pattern selected, Space loops that pattern, as the Play/Pause key does.
- [ ] D3 [M] Space works with the Mixer, Transport or Sample tab focused.
- [ ] D4 [M] After clicking Stop with the mouse, Space starts playback (it does not press Stop again).
- [ ] D5 [M] With a Sequencer step focused, Space plays or pauses and the step does not toggle; Enter still toggles it.
- [ ] D6 [M] Holding Space down toggles once.
- [ ] D7 [M] During a recording, Space stops playback and the take appears on its track.

## E. Lasso and selection (M6, M8, M10)

The press logic for E1, E2 and E4 is covered by `src/views/playlist/sketch/selection.spec.ts` ("a shift press").

- [ ] E1 [M] Shift+drag starting on a clip draws a selection box once the mouse has moved 10 px, and that clip is selected.
- [ ] E2 [M] Shift+click on a clip without moving toggles it, as before.
- [ ] E3 [M] A plain drag on empty space still pans; a plain drag on a clip still moves it; a plain click on a clip still selects nothing.
- [ ] E4 [M] A second lasso adds to the first selection.
- [ ] E5 [M] Cmd+A with the Playlist focused selects every clip. In a text field it selects the text.

## F. Trim (M7)

- [ ] F1 [M] Within 6 px inside either edge of a clip the cursor becomes the horizontal-resize cursor and a 2 px sky bar appears on that edge. (Zone widths covered by `edges.spec.ts`.) Try it on a clip you have just dropped in: that case was broken until the verifier pass.
- [ ] F2 [M] On a clip 12 px wide, the middle third still moves it. (Covered by `edges.spec.ts`.)
- [x] F3 [E] Dragging the right edge of a 4-bar clip to bar 2 with Snap on leaves a 2-bar clip whose last rendered frame is the same source sample that played at bar 2 before. — `audiograph::tests::test_trimming_to_a_bar_line_keeps_the_sound_either_side_of_it` (the test trims to the start of bar 2, leaving one bar, and to the end of bar 2, leaving two)
- [x] F4 [E] Dragging the left edge of that clip from bar 1 to bar 2 leaves a clip starting at bar 2 whose first rendered frame is the source sample that played at bar 2 before. — the same test
- [ ] F5 [M] While dragging, the clip resizes live and the waveform under the mouse does not slide.
- [ ] F6 [M] The right edge cannot be dragged past the end of the audio, and the left edge cannot be dragged before its start or before bar 1. (Limits covered by `edges.spec.ts`.)
- [x] F7 [U] A clip cannot be made shorter than 10 ms; a pattern clip not shorter than one sixteenth-note step. — `edges.spec.ts` ("never makes a clip shorter than it can be"); the engine's own backstop: `audiograph::tests::test_trim_limits`, `test_a_pattern_clip_plays_the_hits_in_its_window`
- [ ] F8 [M] With Option held the edge follows the mouse exactly; released, it snaps again. (Snap bypass covered by `edges.spec.ts`.)
- [ ] F9 [M] Stopped, with the playhead at beat 2.3 and the mouse at beat 2.35, the edge lands on the playhead. With the mouse at beat 2.9 it lands on beat 3. While playing, the playhead is not a snap point. (The rule is covered by `edges.spec.ts` with these numbers.)
- [x] F10 [E] A clip with a 100 ms fade-in, trimmed 1 second in from the left, fades in over the first 100 ms after its new start. — `audiograph::tests::test_trimming_to_a_bar_line_keeps_the_sound_either_side_of_it`, `clip_reader::tests::test_fades_are_measured_from_the_window` (the test trims two seconds in rather than one)
- [ ] F11 [M] With three clips selected, dragging one's right edge left by one beat shortens all three by one beat; one that reaches its minimum stops while the others continue. (Covered by `edges.spec.ts`.)
- [ ] F12 [M] Escape during the drag restores the clip. Releasing outside the canvas applies the trim.
- [ ] F13 [M] A trimmed edge shows a small inward triangle in the label band; an untrimmed edge does not. A trimmed clip does not gain the amber dot.
- [ ] F14 [M] The Sample tab shows the whole file with the played part highlighted, the rest dimmed, and the fade handles inside the highlight. Reset leaves the trim alone.
- [ ] F15 [M] A trimmed pattern clip shows only the steps in its window and plays only the hits that start inside it. (Playing covered by `audiograph::tests::test_a_pattern_clip_plays_the_hits_in_its_window`.)
- [ ] F16 [M] Trimming is one undo step, including a multi-clip trim. (Engine side covered by `clip_commands::tests::test_trimming_clips_of_both_kinds_is_one_step`.)
- [ ] F17 [M] Trimming a clip while it is sounding changes nothing audible until the clip next starts.
- [ ] F18 [M] In the Cut tool, edges cannot be dragged. (Arrives with the Cut tool in M9.)

## G. Stretch (M8)

- [x] G1 [E] Shift+dragging the right edge of a 2-bar clip that starts at bar 1 out to the start of bar 5 (twice its length) sets its Pitch to −12.00 and it renders twice as many frames, within one frame. — `audiograph::tests::test_a_clip_stretched_to_twice_its_length_plays_twice_as_many_frames` (44.1 and 48 kHz files on a 48 kHz engine), `clip_commands::tests::test_stretching_clips_is_one_step`; the drag-to-pitch maths: `edges.spec.ts` ("turns a length into a pitch, and back")
- [ ] G2 [M] Shift+dragging the left edge moves the start to the cursor and leaves the end where it was. (Covered by `edges.spec.ts` and `clip_commands::tests::test_stretching_clips_is_one_step`.)
- [ ] G3 [M] The edge stops when Pitch reaches ±24 semitones. (Covered by `edges.spec.ts`.)
- [ ] G4 [M] The Sample tab's Pitch shows the new value, the clip has the amber dot, and setting Pitch to 0 restores the unstretched length.
- [ ] G5 [M] Stretching a trimmed clip squeezes only the trimmed content; no hidden audio is revealed.
- [ ] G6 [M] Pressing or releasing Shift mid-drag does not change what the drag does.
- [ ] G7 [M] With several clips selected, all sample clips change by the same ratio; pattern clips do not change. (Covered by `edges.spec.ts`; the engine refuses a pattern clip: `test_stretching_clips_is_one_step`.)
- [ ] G8 [M] Shift+dragging a pattern clip's edge does nothing.
- [ ] G9 [M] A Shift+drag starting in an edge zone stretches; starting elsewhere on the clip it draws a lasso; a Shift+click in an edge zone toggles the clip.

## H. Playlist tray, tools, Cut (M9)

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

## I. Clipboard and Repeat (M10)

- [ ] I1 [M] Select two clips on tracks 1 and 3, Cmd+C, click the ruler at bar 9, Cmd+V: copies appear on tracks 1 and 3, the earlier one starting at bar 9, the other at the same distance after it, and they are selected.
- [ ] I2 [M] Cmd+V again puts a second set in the same place.
- [ ] I3 [M] Cmd+X removes the clips and nothing else moves.
- [ ] I4 [E] Changing a pasted clip's trim or settings leaves the original's rendered output unchanged, and no file is written by a paste.
- [ ] I5 [U] A paste in which any clip would start past bar 64 pastes nothing; the notice banner explains.
- [ ] I6 [M] While playing with Snap on, Cmd+V lands on the grid line nearest the playhead at the moment of the press.
- [ ] I7 [M] Copy a clip, open another project, Cmd+V: nothing is pasted.
- [ ] I8 [M] Copy text in another app, copy clips in Dawesome, paste in the other app: the text is pasted.
- [ ] I9 [M] In a rename field, Cmd+C, Cmd+X and Cmd+V act on the text and no clip changes.
- [ ] I10 [U] Repeat on a selection spanning beats 2 to 5.7 puts the duplicate's earliest clip at beat 5.7 exactly, on the same tracks, with the same gaps.
- [ ] I11 [M] Pressing Cmd+R three times lays three copies end to end, the last one selected.
- [ ] I12 [M] Cmd+R never reloads the window, in `pnpm dev` and in a release build, with any view focused.
- [ ] I13 [M] Paste, Cmd+X and Repeat are each one undo step.
- [ ] I14 [M] A pattern clip can be copied and pasted; the paste plays the same pattern.

## J. Context menus (M10, M11, M13)

- [ ] J1 [M] A sample clip's menu reads: Cut, Copy, Paste, Repeat; divider; Split at playhead, Join, Deep copy; divider; Delete. Paste is absent with an empty clipboard, Join with fewer than two clips selected, Split when the playhead does not cross the clip.
- [ ] J2 [M] A pattern clip's menu has "Explode into tracks" and "Open in Sequencer" above those.
- [ ] J3 [M] Right-clicking one of three selected clips and choosing Delete removes all three.
- [ ] J4 [M] Right-clicking an unselected clip leaves the selection as it was, and Delete removes only that clip.
- [ ] J5 [M] Right-clicking empty space at bar 5 on track 2 with clips on the clipboard opens a menu with Paste; choosing it puts the earliest clip at bar 5 with the topmost copied clip on track 2. With an empty clipboard no menu opens.
- [ ] J6 [M] The menu looks as it did, apart from the new items and divider lines.

## K. Deep copy (M11)

- [ ] K1 [M] "Deep copy" on a trimmed, pitched clip: the clip stays where it was, its label becomes `<name>_copy_1.wav`, the Sample tab shows Pitch 0 and no fades, and a file of that name exists in `<project>/clips/`.
- [ ] K2 [E] The copy rendered back has the same number of frames as the original clip rendered, and every sample is within one 16-bit step.
- [ ] K3 [E] With gain +12 dB on a loud file, the copy equals the original clipped to full scale, within one 16-bit step.
- [ ] K4 [E] The file is WAV, 32-bit float, stereo, at the engine's rate.
- [ ] K5 [E] A second deep copy of the same source is named `_copy_2`; an existing `Kick_Copy_1.wav` is not overwritten by `kick_copy_1.wav`.
- [ ] K6 [M] In a never-saved project, Deep copy opens the Save dialog; after saving, the copy is made; cancelling leaves the clip unchanged and writes nothing.
- [ ] K7 [M] Undo puts the clip back on its original file with its trim and pitch; the new file is still in `clips/`. Redo uses that file and no `_copy_2` appears.
- [ ] K8 [M] Deep copy on three selected clips writes three files and is one undo step.
- [ ] K9 [E] If the second of three files cannot be written, no clip changes and no new file remains.
- [ ] K10 [M] A failure shows its reason in the notice banner.
- [ ] K11 [M] Save As to a new folder copies `clips/` there, and the new project's clips play from the copies.
- [ ] K12 [M] Deep copy on a pattern clip turns it into a sample clip of the same length named `<pattern>_copy_1.wav`; undo turns it back.
- [ ] K13 [E] A one-minute clip is written in under 2 seconds.
- [ ] K14 [M] The source file the clip used before is still on disk, unmoved.

## L. Clips page (M12)

- [ ] L1 [M] The sidebar rail has a Clips key below Recordings; its page lists the files in `clips/`.
- [ ] L2 [M] Pressing a file plays it; dragging it onto a track places a clip.
- [ ] L3 [M] Renaming a file that a clip uses renames it on disk and the clip keeps playing, with the new label.
- [ ] L4 [M] Deleting a file in use is refused with the Recordings page's message; deleting an unused file moves it to the Trash.
- [ ] L5 [M] Folders can be created and files moved into them, and clips using those files keep playing.
- [ ] L6 [M] The Recordings page behaves exactly as before.

## M. Join (M13)

- [ ] M1 [E] Cut a clip and join the halves: one clip that renders identically to the original, and no file is written.
- [ ] M2 [E] Join two different samples with a one-beat gap on one track: one clip of `<first>_join_1.wav` whose render equals the two originals' summed render over the span, within one 16-bit step.
- [ ] M3 [M] Shift+J with clips on two tracks does nothing and the notice banner says Join needs one track.
- [ ] M4 [M] Shift+J with a sample clip and a pattern clip selected does nothing and the banner says Join needs one kind of clip.
- [ ] M5 [E] Join two 1-bar pattern clips with a bar between them: one clip of a new 3-bar pattern named "a + b", whose hits fall at the same times as before; the patterns a and b are unchanged.
- [ ] M6 [M] The Sequencer shows the 3-bar pattern with none of the 1 / 2 / 4 keys lit.
- [ ] M7 [M] Pattern clips offset by less than a sixteenth note from each other are not joined, and the banner says why.
- [ ] M8 [M] After a join the joined clip is selected. Undo restores the original clips (and removes a new pattern).
- [ ] M9 [M] Join appears in a clip's menu and the Edit menu, and does the same as Shift+J.

## N. Shortcuts (M5–M13)

- [ ] N1 [U] On macOS, Ctrl+C, Ctrl+V, Ctrl+X, Ctrl+R, Ctrl+A and Ctrl+Z do nothing to clips.
- [ ] N2 [M] Every row of the table in spec §18 behaves as listed.
- [ ] N3 [M] Shift+E, Delete, Cmd+N, Cmd+O, Cmd+S, Shift+Cmd+S, Shift+Cmd+M and Shift+Cmd+E behave as before.

## O. Performance and look (M14)

- [ ] O1 [M] With the 500-clip test project, the frame-rate readout stays at 50 or above during an edge drag, a cut hover and a lasso.
- [ ] O2 [E] Batched add, delete, set-window and split on a 500-clip project each return within 50 ms.
- [ ] O3 [M] No existing control, menu or colour looks different from before, apart from the snap controls' new place.

---

## Things that must never happen

Each of these is a failure whenever it is observed, at any milestone.

- [ ] X1 [E] Editing one clip (trim, stretch, cut, settings, deep copy) changes what another clip plays. (Trim, cut and settings proved by `audiograph::tests::test_changing_a_clip_changes_no_other_clip_of_the_sample`; a stretch is a change of the Pitch setting, which that test makes; deep copy to be proved in M11.)
- [ ] X2 [E] A deep copy or join writes a file anywhere outside the project's folder.
- [ ] X3 [E] A deep copy or join overwrites an existing file.
- [ ] X4 [E] Undo, redo, deep copy or join deletes or moves an audio file. (Undo and redo proved by `history::tests::test_undo_and_redo_leave_files_alone`; deep copy and join to be proved in M11 and M13.)
- [ ] X5 [M] Space starts or stops playback while typing in a text field, the tempo field, a rename field, search or a drop-down list.
- [ ] X6 [M] C, S, Shift+J, Cmd+C, Cmd+X, Cmd+V, Cmd+R or Cmd+A changes the project while typing in a text field.
- [ ] X7 [M] Cmd+R reloads the app window.
- [ ] X8 [E] A failed deep copy or join leaves a clip pointing at a missing or partly written file.
- [x] X9 [E] An older-format project is rewritten or changed merely by opening it. — `project::tests::test_an_older_file_loads_whole_and_is_left_alone`
- [x] X10 [E] A project with a trimmed clip is written with a version lower than 3 (which would let an older build open it and play the clip untrimmed). — `project::tests::test_the_version_follows_what_a_project_uses`
- [x] X11 [E] A whole, unedited clip sounds different from how it sounded before this project, beyond one 16-bit step. — `clip_reader::tests::test_a_whole_clip_is_read_as_it_always_was`
- [ ] X12 [E] A paste, repeat or multi-clip cut applies to only some of its clips after a failure (each is all or nothing).
- [ ] X13 [M] A trim, cut or stretch moves a clip the user did not select or grab.
- [ ] X14 [M] `docs/todo.md` appears in any commit from this project.
