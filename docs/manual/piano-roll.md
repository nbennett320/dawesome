---
title: Piano roll
---

# 8 Piano roll

<!-- PROVISIONAL -->

The **Piano Roll** tab shows the selected pattern's notes on a grid of 16th-note
steps, against a piano keyboard. Notes are played by an instrument plugin on
a track (see [9 Instruments and effects](./plugins.md)).

## 8-1 Opening the Piano Roll

Do one of the following:

- Choose **Window ▸ Piano Roll**.
- In the Sequencer, click **Piano roll**.

The Piano Roll shows the same pattern as the Sequencer. If the project has no
pattern, click **New pattern**.

![The Piano Roll tab](./img/piano-roll.png)
<!-- SCREENSHOT: Piano Roll with a few notes, one selected, numbered 1–8 -->

1. **Pattern** — Which pattern is shown.
2. **Plays on** — The track whose instrument plays the notes while the
   pattern plays on its own, and when you audition notes. Shows **No
   instrument** until a track has one.
3. **Play** — Loops the pattern on its own. Click again to stop. Its light is
   lit while it plays.
4. **Length** — The pattern's length in bars. Change it in the Sequencer
   (see [7-2-3 Pattern length](./patterns.md)).
5. Ruler — A number at each bar.
6. Keyboard — Each C is named (middle C is C4).
7. Grid — One row per semitone, one column per step. Beats and bars are
   drawn stronger; the area past the pattern's end is shaded.
8. Notes — Selected notes are drawn in the accent color.

::: tip NOTE
To hear notes, a track needs an instrument. Drag one from the sidebar onto a
track in the Mixer. Until then the Piano Roll shows "Drag an instrument onto a
track in the Mixer to hear notes".
:::

## 8-2 Drawing notes

- *Click* an empty place on the grid to draw a note there. It starts on the
  step you clicked and is as long as the last note you drew or resized (a
  quarter note at first).
- *Drag* right while drawing to set the note's length.

You hear each note as you draw it.

Notes always start on a whole step and are whole steps long. There is no
snap setting in the Piano Roll. Notes can't run past the pattern's end.

## 8-3 Selecting notes

| To | Do this |
| --- | --- |
| Select a note | *Click* it |
| Add or remove a note from the selection | *Shift-click* it |
| Select notes with a box | *Shift-drag* on an empty place; touched notes are added to the selection |
| Select every note | **⌘A (Ctrl+A)**, or **Edit ▸ Select All** |
| Clear the selection | **Escape**, or click past the pattern's end |

Keys work while the Piano Roll is the focused pane's tab.

## 8-4 Moving and resizing notes

- **Move**: *drag* a note's body. If the note is selected, every selected
  note moves with it. Notes move by whole steps and semitones, and keep their
  shape at the pattern's edges and the ends of the keyboard. You hear the
  note at each new pitch.
- **Resize**: *drag* a note's right edge (the pointer changes to a
  left-right arrow). Every selected note gets longer or shorter by the same
  number of steps. A note is at least one step long.

## 8-5 Deleting notes

- Select notes and press **Delete** (or **Backspace**).
- *Right-click* a note to delete it.

Each drag, and each delete, is one step to undo with **⌘Z (Ctrl+Z)**.

## 8-6 Velocity

All notes are drawn at the same velocity. Velocity can't be edited yet.

## 8-7 Auditioning

- *Click* a key on the keyboard to hear it while you hold the mouse button.
  *Drag* along the keys to play each in turn.
- Drawing a note, or moving one to a new pitch, plays it briefly.

Notes sound on the **Plays on** track's instrument.

## 8-8 Playing the pattern

Click **Play**, or press **Space** while the Piano Roll is the focused pane's
tab. A line shows where the pattern is. Playback stops when you leave the
Piano Roll and the Sequencer (see [7-3-4 Playing a pattern](./patterns.md)).
