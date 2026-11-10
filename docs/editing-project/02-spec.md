# 02 — Spec: Playlist editing, clipboard and shortcuts

Written 2026-11-10. This is the agreed behaviour of every feature. Each statement cites the answer it comes from: `Q…` is in `01-answers.md`; `F…`, `G…` and `H…` are in `01b-follow-ups.md`. A statement with no citation is a bug in this spec.

Terms used throughout:

- **Sample clip:** a clip that plays an audio file. **Pattern clip:** a clip that plays a Sequencer pattern.
- **Window:** the part of its file or pattern that a clip plays.
- **Playlist-focused:** the Playlist is the focused view and the key press is not going into a text field (the existing rule for Delete and Shift+E).
- **Snap grid:** the existing snap setting (Snap key and snap value).
- **Notice banner:** the existing dismissable banner (`ProjectNotice`).

---

## 1. Scope

Built in this project:

1. Trim and stretch by dragging clip edges.
2. Cut tool, and "Split at playhead".
3. Deep copy, with a `clips/` folder and a Clips sidebar page.
4. Clipboard: copy, cut, paste, Repeat, Select All.
5. Lasso that can start over a clip.
6. Shortcuts: Space, C, S, Shift+J, and the clipboard and undo keys.
7. Playlist tools (Select, Cut) in a tray inside the Playlist.
8. Join.

Groundwork your answers added:

- A 64-bar playlist, with the loop ending at the last clip (Q7, Q7a, F2).
- Clips and the playhead keep their beat when the tempo changes (Q8, F6).
- Undo and redo for the whole project (Q10, Q11).
- An Edit menu (Q135).

Not built (each was asked and declined, or is recorded in `later.md`): pitch-preserving time-stretch (Q72), audio scrubbing while dragging (Q62), sideways auto-scroll (Q71), an unsaved-changes indicator (Q20), Cmd+click selection (Q45), touch-screen support (Q46), numeric trim or stretch fields (Q68), clip names or colours (Q96), restyling of any existing control or menu (Q153, Q165), cleaning the lint and typecheck baseline (Q5), platforms other than macOS (Q4).

---

## 2. Timeline groundwork

### 2.1 Playlist length

- The playlist is 64 bars long (Q7, Q7a). A clip's start is kept within it, as it is kept within 4 bars today.
- A clip's right end may hang past the playlist's last beat (Q58).
- At launch, and after New Project or Open, the Playlist shows the first 4 bars, scrolled to the start (F3, H3). You can scroll right and zoom out as far as the whole 64 bars (F3).
- When zoomed out, beat lines are hidden once they would be closer than about 8 px, bar lines stay, and ruler numbers are thinned so they never overlap (F4).

### 2.2 Loop

- With Loop on, the loop runs from the start to the end of the last clip, rounded up to the next bar. An empty playlist loops 4 bars (F2).
- While Loop is on, a small marker on the ruler shows the loop's end, in an existing theme colour (F39).
- While a recording is in progress, the loop's end stays where it was when the recording started (F40).

### 2.3 Tempo

- When the tempo changes, every clip keeps its beat position, in the engine and on the canvas (Q8). The playhead keeps its beat too (F6).
- A sample clip's length is an amount of audio. A tempo change does not change what a trimmed or stretched clip plays or how long it lasts in seconds (Q9, Q88).
- If the playlist is playing when the tempo changes, playback carries on from the beat it had reached at the new tempo, with a very short fade (F5).
- The tempo cannot be changed while a recording is in progress (F12).

---

## 3. Data model

- Two clips are "instances of the same source" when they point at the same audio file. There is no other link between them (Q21).
- A trimmed sample clip is its file plus two per-clip values: where in the file it starts and how much of the file it plays. The file is never rewritten (Q22).
- A trimmed pattern clip is a window on its pattern in the same way (Q26, F13).
- No edit to one clip changes any other clip, including clips of the same file and the other half of a cut (Q23).
- Clips have no name or colour property. A sample clip's label is its file name (Q96).

### 3.1 Project file

- A project is written as format version 3 only if some clip is trimmed. Otherwise it is written as it is today (version 1 or 2). Older builds refuse version 3 files with the existing "saved by a newer version" message (Q24).
- Existing version 1 and 2 projects open with every clip untrimmed, and nothing in the file is rewritten until you save (Q25).

---

## 4. Undo and redo

### 4.1 What is undoable

- Adding, moving and deleting clips; clip settings (gain, pan, pitch, attack, decay); exploding a pattern clip; mixer changes (volume, pan, mute, solo, track names, reset); Sequencer edits (rows, patterns, steps, bars, names, folders); tempo; and every new operation in this spec (Q10, Q11).
- Not undoable: clips placed by a finished recording; the metronome, loop, snap and count-in toggles and the snap value; track arming (Q11b). File operations in the Recordings and Clips pages (F9).
- Never undo steps: selection changes, tool changes, zoom, scroll, playhead moves, play and stop, tab and pane layout (Q17).

### 4.2 Steps

One gesture is one step (Q14):

| Gesture | Steps |
|---|---|
| One edge drag, trim or stretch, however many clips it changes | 1 |
| One cut, however many clips it cuts | 1 |
| One Paste, Cmd+X or Repeat | 1 |
| One Deep copy, however many clips (G2) | 1 |
| One Join | 1 |
| One clip or selection move | 1 |
| Deleting a selection | 1 |
| Dropping one clip in | 1 |
| One knob or fader drag, press to release | 1 |
| One mouse stroke painting Sequencer steps | 1 |
| One visit to the tempo field, from focus to leaving it | 1 |
| Each mute, solo, rename, add or remove | 1 |

### 4.3 Rules

- Cmd+Z undoes; Shift+Cmd+Z redoes (Q13). They act on the project whenever you are not typing in a text field, whichever view is focused. In a text field they undo typing (Q19).
- The history holds 100 steps (Q15).
- It is cleared by New Project and Open, survives Save, and is not written to the project file (Q16).
- After you undo and then make a new edit, the undone steps can no longer be redone (F11).
- After an undo or redo, the clips it changed or restored become the Playlist selection (Q18).
- Undo and redo do nothing while a recording is in progress or while a mouse drag is in progress (F8).
- A finished recording leaves the history intact; its clips stay as if they had always been there (F7).
- Undo never deletes a file on disk (Q119, F9).
- The history lives in the engine. The frontend asks the engine to undo or redo and redraws from what the engine returns (Q12).

---

## 5. Edit menu

- The menu bar gains an Edit menu: Undo, Redo; divider; Cut, Copy, Paste, Repeat, Select All; divider; Join (Q135, F10).
- Undo and Redo keep plain labels (F10).
- The items act on clips when the Playlist is focused and on text while typing in a text field (Q135, Q134).

---

## 6. Playlist tray and tools

### 6.1 Tray

- A strip above the ruler, inside the Playlist pane; the canvas starts below it (Q32, Q32a).
- From the left: the Select and Cut tool keys, a gap, then the Snap key and the snap-value drop-down. It uses the `ft-tray` style and `LatchKey`s (F36).
- The Snap key and snap-value drop-down move here from the topbar. The topbar keeps Metronome, Count-in and Loop (Q32b, F36).
- Each tool key is a `LatchKey` with an icon: an arrow for Select, a razor blade for Cut. The active tool's LED is lit (Q33).

### 6.2 Tools

- There are exactly two tools: Select (the default) and Cut. Trim and stretch are edge drags inside the Select tool (Q30).
- The tool is Select at launch and after opening a project. It is not saved (Q34).
- S switches to Select and C switches to Cut, when pressed with no modifier while Playlist-focused. Caps Lock makes no difference (Q38).
- Cut is sticky: it stays until S is pressed (Q35). Escape does not leave the Cut tool; it only clears the selection (Q36).

### 6.3 Select tool

Everything the mouse does today stays: a plain drag on empty space pans, a plain drag on a clip moves it, Shift selects, right-click opens the menu, double-click opens the clip (Q31). A plain click does not select a clip (Q44). Trackpad scrolling and pinch are unchanged (Q46). Added: edge drags (sections 8 and 9) and the lasso change (section 7).

### 6.4 Cut tool

- Clicking a clip cuts it (section 10). The clip does not need to be selected (Q36).
- Dragging empty space pans, the wheel scrolls and zooms, right-click opens the menu, and Shift selects, all as in the Select tool. Clips cannot be moved or trimmed (Q37).

---

## 7. Selection and lasso

- Holding Shift is the selection modifier, as now (Q28).
- Shift+drag starts a lasso anywhere, including over a clip, once the mouse has moved 10 px (Q39, Q40). Shift+click without moving still toggles the clip (Q39).
- A lasso started over a clip selects that clip, because the box touches it (Q41).
- A lasso always adds to the selection (Q42). Its hit rule is unchanged: the box touches the clip's lane-height box (Q43).
- With Shift held, a drag that starts in a clip's edge zone stretches (section 9) instead of starting a lasso; a Shift+click with no drag toggles the clip even in an edge zone (Q81).
- Cmd+A selects every clip, when Playlist-focused (Q136, Q162).

---

## 8. Trim

### 8.1 Grabbing an edge

- Each edge has a grab zone 6 screen pixels wide, inside the clip, at every zoom level (Q47), running the clip's full height (Q49).
- On a narrow clip each zone shrinks to at most a third of the clip's width, so the middle third always moves the clip (Q48).
- Over a zone the cursor is `ew-resize` (Q50) and a 2 px bar in the sky accent colour is drawn on that edge (Q51).
- Pressing an edge zone affects the selection exactly as pressing the clip's middle does: pressing an unselected clip clears the selection and does not select it (Q65, Q44).
- Edge drags exist only in the Select tool (Q37).

### 8.2 Dragging

- A plain drag of an edge trims: it moves the clip's start or end (Q22).
- The audio stays where it is on the timeline. Dragging the left edge right hides the start; dragging it back reveals it (Q57).
- The clip resizes live under the mouse, revealing or hiding waveform. The engine is told once, on release (Q52).
- Nothing is heard while dragging (Q62).
- Whether Shift was held is decided at the press and does not change during the drag (Q82).

### 8.3 Limits

- The left edge stops at the source's first sample or at beat 0, whichever comes first (Q57). An edge stops at the end of the source audio (Q56).
- The right edge may go past the playlist's last beat, as far as the source allows (Q58).
- A sample clip cannot be shorter than 10 ms of played audio (Q55). A pattern clip cannot be shorter than one sixteenth-note step (F15).
- A trim may make a clip overlap its neighbour; both play (Q59).

### 8.4 Snapping

- With Snap on, the dragged edge's timeline position snaps to the nearest snap step (Q53).
- The playhead is also a snap point: the edge goes to the playhead when the cursor is nearer to it than to either neighbouring grid line. This applies only with Snap on, Option not held, and playback stopped or paused (Q91b, F32).
- Holding Option bypasses snap for trim, stretch and cut. Existing clip moves and drops are unaffected (Q54).

### 8.5 Fades

- Fades follow the trimmed edges: the fade-in starts at the clip's current left edge and the fade-out ends at its current right edge (Q60).
- If a trim makes the clip shorter than a fade, the stored fade is shortened to fit, as it is today. Dragging the edge back out does not restore it; undo does (Q61).

### 8.6 Several clips

- When the grabbed clip is part of a selection, every selected clip moves the same edge by the same amount of time (Q64, F33).
- Only the grabbed clip's edge snaps; the others move by that same amount (F33).
- Each clip stops at its own source boundary and minimum length (Q64, F33). Pattern clips in the selection trim along with the rest (F33).

### 8.7 Ending the drag

- Releasing applies the trim, including when the mouse is outside the canvas (Q70).
- Escape during the drag puts the clip back and sends nothing to the engine (Q69).
- The view does not auto-scroll sideways (Q71).
- An edit that lands while the clip is sounding is heard from the clip's next start (Q63).

### 8.8 Appearance

- An edge that hides audio or steps beyond it shows a small solid triangle in the label band at that edge, pointing inward, in the clip's border colour (Q66, F29).
- A trim does not show the amber "modified" dot (Q66).
- In the Sample tab, a trimmed clip shows the whole file with the played part highlighted and the rest dimmed. The fade envelope and its handles sit inside the highlight. The Length readout shows the played length. The highlight cannot be dragged (Q67, F30).
- The Sample tab's Reset returns the five settings only; trim is left alone (Q87).
- There are no numeric trim fields (Q68).

---

## 9. Stretch

- Shift+dragging an edge stretches the clip to the cursor (Q79, Q81).
- Stretch is tape-style: it changes speed and pitch together (Q72). It works by writing the clip's Pitch setting, which the Sample tab shows (Q77).
- The opposite edge stays where it is (Q79).
- Stretch applies to what the clip currently plays; it does not reveal trimmed-away audio (Q80).
- The limit is Pitch's ±24 semitones: a quarter to four times the unpitched length. The edge stops there (Q77).
- It snaps like a trim: grid, playhead and Option bypass (Q83, F32).
- The clip and its waveform squeeze or expand live; the engine is told on release (Q84).
- A stretched clip shows the amber "modified" dot, because its Pitch is no longer 0. There is no other indicator (Q85).
- A stretch is removed by setting Pitch back to 0 in the Sample tab, or by undo. There is no menu item for it (Q86).
- A stretched clip keeps its length in seconds when the tempo changes (Q88).
- When the grabbed clip is part of a selection, every selected sample clip is stretched by the same ratio, each keeping its own opposite edge fixed and stopping at its own limit. Pattern clips in the selection are left as they are (Q89, F34).
- Stretch does not apply to pattern clips (Q26). A Shift+drag on a pattern clip's edge does nothing (Q27).
- Escape, release outside the canvas, minimum length, overlap and "heard from the next start" follow section 8 (Q69, Q70, Q55, Q59, Q63).

---

## 10. Cut

### 10.1 Ways to cut

- In the Cut tool, clicking a clip splits it at the click position (Q90).
- "Split at playhead" in a clip's context menu splits at the playhead (Q90). While playing it uses the playhead's position at the moment you choose it, unsnapped. The item is left out when the playhead does not cross the clip (F31).
- There is no cut shortcut (Q90).

### 10.2 Cut tool behaviour

- The cut lands at the mouse position, snapped to the grid when Snap is on (Q91).
- The playhead is also a snap point, under the same rule as section 8.4: Snap on, Option not held, playback stopped or paused (Q91, Q91a, Q91c).
- Holding Option bypasses snap (Q54).
- A vertical dashed line in the accent colour on the hovered clip shows where the cut will land (Q92).
- The cursor is a custom razor blade, drawn as an SVG in the existing icon style and approved by you, with the crosshair as its fallback (Q93, Q93a).
- The cut happens on release, if the mouse has not moved more than 10 px since the press. Moving away first cancels (Q100).
- If the position is closer to an edge than the minimum clip length, no cut is made and no line is shown (Q99).
- The clip under the cursor is always cut. If it is part of a selection, every other selected clip crossing that position is cut too (Q98).

### 10.3 Result

- A cut produces two clips that are windows on the same file or pattern (Q22, F19).
- The left half is the original clip: it stays selected and inspected if it was. The right half is a new, unselected clip (Q97).
- Both halves keep the original's track, gain, pan and pitch (Q96).
- The left half keeps the fade-in and has no fade-out. The right half keeps the fade-out and has no fade-in (Q94).
- A fade longer than the half it lands on is shortened to that half (Q95).
- Played back, the two halves produce exactly the same output frames as the uncut clip, at any pitch and any file sample rate, except where a fade was shortened under the rule above (F35).

---

## 11. Deep copy

### 11.1 Command

- "Deep copy" is an item in the clip's context menu, with that exact label and no shortcut (Q122). It is offered on every clip it applies to (Q102), including pattern clips (Q26).
- It replaces the clip in place: the same clip, at the same position and track, now plays the new file (Q103).
- Chosen on a clip inside a selection, it applies to every selected clip. Each gets its own new file. It is one undo step, and if any file fails, no clip is changed (Q150, G2).

### 11.2 What the file contains

- The clip as it plays: trim, pitch and speed, fades, pan and gain are all baked in. The track's mixer strip and the master are not (Q104).
- Samples above full scale are clipped to full scale in the file (Q105).
- Afterwards the clip's settings are neutral (no trim, Pitch 0, no fades, centre pan, 0 dB), so nothing is applied twice (Q104).
- The file is exactly as long as the clip's played length, with no added silence or tail (Q121).
- Format: WAV, 32-bit float (Q108), always stereo (Q109), at the engine's current output rate, or the source file's rate if no output device is open (Q107).

### 11.3 Accuracy

Played back, the copy has the same number of frames as the original clip, and every sample is within one 16-bit step (about 0.00003 of full scale) of the original, except samples the original pushes above full scale, which the copy clips (Q106).

### 11.4 Files

- Files go in `clips/` inside the project folder. For an older project that is a bare `.dawesome` file, `clips/` sits beside that file (Q114).
- Name: `<source name>_copy_<n>.wav` (Q110). The next free number is used, names are compared without regard to case, and no existing file is ever overwritten (Q111).
- The old source file is never deleted or moved (Q112).
- In a project that has never been saved, the Save dialog opens first. The deep copy happens after the save and is abandoned if you cancel (Q113).
- Save As copies `clips/` into the new project folder, and the new project's clips point at the copies (Q115).
- The project is not saved automatically afterwards (Q120).

### 11.5 Feedback

- If writing takes longer than about half a second, the mouse shows the wait cursor until it finishes. Otherwise nothing is shown (Q117).
- On any failure the clip is left exactly as it was, any partly written file is removed, and the reason is shown in the notice banner in its error style (Q118, F37).
- An edit that lands while the clip is sounding is heard from its next start (Q63).
- A one-minute clip is written in under 2 seconds on your Mac (Q164).

### 11.6 Undo

Undo puts the clip back on its original file with its original trim and settings. The new file stays on disk. Redo reuses that file and does not write another (Q119).

### 11.7 Pattern clips

- The clip is rendered as it plays at that moment: current steps, row volumes applied, muted rows silent, at the current tempo (F21).
- The file is exactly the clip's length; any hit still ringing at the end is faded out over 10 ms (F20).
- Name: `<pattern name>_copy_<n>.wav` (F21).
- Afterwards the clip is an ordinary sample clip, no longer linked to the pattern. Undo turns it back into the pattern clip (F21).

---

## 12. Clips sidebar page

- The sidebar gains a "Clips" page with a key on the rail below Samples, Patterns and Recordings (Q116, F28). Its icon is drawn in the existing line style and approved by you (F28).
- It behaves exactly like the Recordings page does today, pointed at `clips/`: search, preview on press, dragging into the Playlist or Sequencer, the "in use" marker, renaming with clips that use the file following along, deleting to the Trash (refused while the project uses the file), and folders (Q116a, F27).
- Files made by Join appear there too (F27).
- Its file operations are not undo steps (F9).

---

## 13. Clipboard and Repeat

### 13.1 Copy and cut

- Cmd+C copies every selected clip, sample and pattern, with its track, its position relative to the others, its trim and its settings (Q123, Q26).
- Cmd+X does the same and removes the clips, leaving a gap; nothing else moves (Q131).
- With nothing selected, Cmd+C and Cmd+X do nothing (Q123).
- The clip clipboard is private to Dawesome. It does not change the system clipboard. In a text field, Cmd+C, Cmd+V and Cmd+X act on the text (Q134).
- New Project and Open empty the clipboard (Q133).

### 13.2 Paste (Cmd+V)

- The earliest copied clip starts at the playhead and the rest keep their spacing (Q124).
- The position is snapped to the grid when Snap is on and exact when Snap is off, whether playing, stopped or paused. While playing, the playhead's position at the moment of the key press is used (Q125, G1).
- Clips go on the tracks they were copied from (Q126).
- A paste may overlap existing clips (Q127).
- If any pasted clip would start past the end of the playlist, nothing is pasted and the notice banner says why (Q128, F37).
- Pressing Cmd+V again pastes in the same place (Q129).
- The pasted clips become the selection (Q130).
- A pasted sample clip plays the same file as its original and has its own independent trim and settings. A pasted pattern clip is linked to the same pattern. No file is written (Q132).
- A pattern clip whose pattern no longer exists is skipped; the rest is pasted (H2).
- With an empty clipboard, Cmd+V does nothing (F38).

### 13.3 Repeat (Cmd+R)

- Repeat duplicates the selection so that the duplicate starts exactly where the selection's span ends. The span runs from the earliest start to the latest end. There is no rounding (Q137).
- Duplicates go on the same tracks as their originals and keep the gaps between them (Q139).
- The new duplicates become the selection (Q138).
- Past the end of the playlist, nothing is repeated and the notice banner says why (Q139, Q128).
- Duplicates share audio like pasted clips. Repeat does not change the clipboard. With nothing selected it does nothing (Q139).
- Cmd+R never reloads the app window, in development or release builds, whichever view is focused (Q140).

---

## 14. Join

### 14.1 General

- Shift+J joins the selection into one clip (Q28, Q29). It works on two or more clips (Q29).
- All selected clips must be on one track; otherwise nothing happens and the notice banner says so (Q142, F26, F37).
- A selection holding both sample clips and pattern clips is not joined; the notice banner says Join needs clips of one kind (Q146, F37).
- The joined clip becomes the selection (H1).
- Join is one undo step (Q14).

### 14.2 Sample clips

- If the pieces are neighbouring windows on the same file that still line up, they are rejoined into one clip and no file is written. Otherwise the clips are rendered together into one new file in `clips/` and replaced by one clip (Q141).
- In a bounced join the new clip runs from the earliest start to the latest end. Gaps are silence in the file. Overlapping clips are summed (Q143).
- A bounced join follows the Deep copy rules for format, what is baked, gain clipping, the Save-dialog-first rule, failure handling and undo (Q144).
- Name: `<source name>_join_<n>.wav`, after the first clip's source (Q147).

### 14.3 Pattern clips

- Join makes a new pattern holding the selected clips' steps in sequence and replaces the clips with one clip of it. The original patterns and any other clips of them are untouched (Q145, F26).
- Two halves of a cut pattern clip that still line up are rejoined into one clip of the same pattern, and no new pattern is made (F19).
- The new pattern starts where the first clip starts. Its length is the span rounded up to a whole bar; gaps are empty steps. The new clip is trimmed to the exact span, so nothing moves on the timeline (F23).
- Where the clips overlap, their steps are merged (F26).
- A pattern may be any whole number of bars. The Sequencer's 1 / 2 / 4 keys stay; on a longer pattern none is lit, and pressing one sets that length as now (F22).
- If the clips do not sit on the same sixteenth-note grid as the first clip, the join is refused and the notice banner says why (F24, F37).
- Name: the joined patterns' names in order, joined with " + ". Unnamed patterns use their numbers. More than three names become, for example, "verse + 3 more" (F25).
- The new pattern is placed after the first source pattern, in the same folder (F26).
- Undo removes the new pattern and restores the clips (F26).

---

## 15. Pattern clips: summary

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

---

## 16. Context menus

### 16.1 Clip menu

- Items, in order: Cut (⌘X), Copy (⌘C), Paste (⌘V), Repeat (⌘R); divider; Split at playhead, Join (⇧J), Deep copy; divider; Delete (⌫). A pattern clip keeps "Explode into tracks" and "Open in Sequencer" at the top (Q148, Q149).
- Items that do not apply are left out: Join with fewer than two clips selected (Q148), Split at playhead when the playhead does not cross the clip (F31), Paste when the clipboard is empty (F38).
- Paste from a clip's menu pastes at the playhead on the original tracks, like Cmd+V (Q152a).
- Right-clicking a clip that is part of a selection applies every item to the whole selection, Delete included (Q150).
- Right-clicking a clip that is not selected leaves the selection unchanged and applies the items to that clip only (Q151).
- The menu keeps its current look. Only items and divider lines are added (Q153).

### 16.2 Empty-space menu

- Right-clicking empty space opens a menu with Paste (Q152). With an empty clipboard no menu opens (F38).
- That Paste puts the earliest copied clip at the right-clicked position, snapped when Snap is on. The topmost copied clip goes on the right-clicked track and the others keep their spacing in time and tracks. Overlap and end-of-playlist rules are the same as Cmd+V (Q152a).

---

## 17. Space

- Space does what the topbar Play/Pause key does, including looping the selected pattern when the Sequencer is the focused view (Q154).
- It works everywhere, whichever view is focused, except while typing (Q155).
- While typing in a text field, the tempo field, a rename field, search, a drop-down list or editable text, Space never starts or stops playback (Q156).
- Outside those, Space always means transport. Focused step cells, sidebar rows, wind keys and buttons no longer react to Space; they keep Enter (Q157).
- Holding Space toggles once (Q158).
- During a recording, Space stops playback and finishes the take, as the Play/Pause key does (Q159).

---

## 18. Shortcuts

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

---

## 19. Messages

All short messages from this project appear in the notice banner: info style for refusals (paste or repeat past the end, join needs one track, join needs one kind, join clips not on the same grid), error style for failures (deep copy or join failed) (F37).

---

## 20. Quality bars

- **Performance:** with 500 clips in the project, edge drags, cut previews and lassos stay at 50 frames per second or better on your Mac, and every edit command returns within 50 ms (Q163).
- **Look and feel:** no existing style changes. New canvas marks use colours already in `theme.ts`. New controls are built from existing four-track and sidebar components. Cursors are standard, except the razor blade (Q165).
- **Platform:** verified on macOS only (Q4).

---

## 21. How we work

- I commit at the end of each milestone on `playlist-enhancements`, with one short lowercase line in your style and no trailer lines (Q1, Q2, F1). I never touch `docs/todo.md` (Q3).
- A milestone passes when both test suites pass and my changes add no new eslint or tsc problems (Q5).
- Canvas gestures are verified by unit tests on extracted logic, numeric engine tests, and a manual checklist you run in the app at each milestone (Q6).
