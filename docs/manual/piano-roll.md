---
title: Piano roll
---

# 8 Piano roll

<!-- PROVISIONAL -->

The **Piano Roll** tab shows one row's notes in the selected pattern, on a
grid of 16th-note steps against a piano keyboard. The other rows' notes are
drawn faintly behind them. Any row can be edited here, a sample row or a
synth row (see [7-3 Rows](./patterns.md#_7-3-rows)).


## 8-1 Opening the Piano Roll

Do one of the following:

- Choose **Window ▸ Piano Roll**.
- In the Pattern Sequencer, *right-click* a row's controls and choose
  **Edit in Piano Roll**. The Piano Roll opens on that row, whether it shows
  steps or a note preview.
- In the Pattern Sequencer, *click* a row's note preview (see
  [7-4-2](./patterns.md#_7-4-2-steps-or-a-note-preview)). The Piano Roll
  opens on that row.
- Drop an instrument on a track's strip in the **Mixer** (see
  [9-3-1](./plugins.md#_9-3-1-instruments)). The Piano Roll opens on the new
  synth row.

The Piano Roll shows the same pattern as the Pattern Sequencer. If the
project has no pattern, it says "No pattern yet" and offers **New pattern**.
If it has no rows yet, it says "No rows yet. Drag a sample or an instrument
into the Pattern Sequencer to draw its notes", with a key that opens the
Pattern Sequencer.

![The Piano Roll tab](./img/piano-roll.png)
<!-- SCREENSHOT: retake: Piano Roll on a synth row with ghost notes from sample rows, a few notes, one selected, the shaded area past the pattern's end, numbered 1–9 -->

1. **Pattern** — Which pattern is shown.
2. **Row** — Which row's notes you edit. Its instrument plays the keys and
   the notes you draw. Lists every row, sample and synth.
3. **Play** — Loops the pattern on its own, every row's notes, from the
   playhead. Click again to stop. Its light is lit while it plays.
4. **Bars** — The pattern's length, on the same counter as the Pattern
   Sequencer's (see [7-2-3 Pattern length](./patterns.md#_7-2-3-pattern-length)).
5. Tools and snap — **Select**, **Draw** and **Cut**, **Snap to grid** and
   the snap amount ([8-2](#_8-2-tools-and-snap)).
6. Ruler — A number at each bar, a tick at each beat. *Click* or *drag* here
   to move the playhead ([8-9](#_8-9-playing-the-pattern)). A marker shows
   where the pattern ends.
7. Keyboard — Each C is named (middle C is C4).
8. Grid — One row per semitone, one column per step. Beats and bars are
   drawn stronger. The grid goes on past the pattern's end, which is
   shaded: 4 bars past the end, or past the furthest you have scrolled, up
   to 128 bars.
9. Notes — The row's notes. Selected notes are drawn in the accent color.
   The other rows' notes are drawn faintly behind them; they can't be
   selected or edited here.


## 8-2 Tools and snap

The Piano Roll has the playlist's three mouse tools. Only one is active at a
time; its light is lit.

| Key | Tool | What it does |
| --- | --- | --- |
| **S** | **Select** | Selects, moves and trims notes. A *drag* on empty grid scrolls it; a *click* there clears the selection. |
| **D** | **Draw** | A *click* on empty grid draws a note. On a note, it works as **Select**. |
| **C** | **Cut** | A *click* on a note cuts it in two. |

The keys work while the Piano Roll is the focused pane's tab. With every
tool, *Shift-click* and *Shift-drag* select ([8-4](#_8-4-selecting-notes)).

**Snap to grid** turns snap on or off; its light is lit when on. The menu
beside it sets the grid: **1 bar**, **1/2 bar**, **1 beat**, **1/2 beat** or
**1/4 beat**. The Piano Roll's snap is its own, apart from the playlist's.

With snap off, or while you hold **Option (Alt)**, nothing snaps: notes start,
end, move and cut right where the mouse is, and can be as short as 1/32 of a
step. With snap on, notes are at least a step long.

::: tip NOTE
A row whose notes are off the step grid shows them as a note preview in the
Pattern Sequencer, not as steps (see
[7-4-2](./patterns.md#_7-4-2-steps-or-a-note-preview)).
:::

## 8-3 Drawing notes

With the **Draw** tool:

- *Click* an empty place on the grid to draw a note there. It starts on the
  snap line at or before where you clicked, and is as long as the last note
  you drew or trimmed (a quarter note at first).
- Without letting go, *drag* up or down to move it to another key, and
  *drag* right to set its length.

You hear each note as you draw it.

You can draw past the pattern's end. Moving or trimming a note past it
works the same way. The pattern grows to the end of the bar the note ends
in, up to 128 bars (see
[7-2-3 Pattern length](./patterns.md#_7-2-3-pattern-length)).


## 8-4 Selecting notes

| To | Do this |
| --- | --- |
| Select a note | *Click* it (with **Select** or **Draw**) |
| Add or remove a note from the selection | *Shift-click* it |
| Select notes with a box | *Shift-drag* on an empty place; touched notes replace the selection |
| Select every note | **⌘A (Ctrl+A)**, or **Edit ▸ Select All** |
| Clear the selection | **Escape**, or *click* empty grid with **Select** |

Keys work while the Piano Roll is the focused pane's tab.

## 8-5 Moving, trimming and cutting notes

- **Move**: *drag* a note's body. If the note is selected, every selected
  note moves with it. The notes follow the mouse, with an outline where they
  will land: the nearest snap line and key. You hear the note at each new
  pitch.
- **Trim**: *drag* a note's left or right edge (the pointer changes to a
  left-right arrow, and a bar marks the edge). The same edge of every
  selected note moves by the same amount. A note is at least one step long.
- **Cut**: with the **Cut** tool, *click* a note. A line shows where it
  will be cut: the snap line nearest the mouse. If the note is selected,
  every selected note under that line is cut too. Neither half can be
  shorter than a step.

Press **Escape** while dragging to put the notes back.

## 8-6 Copy, paste and repeat

| Command | Keys | What it does |
| --- | --- | --- |
| Copy | **⌘C (Ctrl+C)** | Copies the selected notes. |
| Cut | **⌘X (Ctrl+X)** | Copies the selected notes and deletes them. |
| Paste | **⌘V (Ctrl+V)** | Pastes the notes, the earliest on the snap line at or before the mouse (or the first snap line in view, when the mouse isn't over the grid). |
| Repeat | **⌘R (Ctrl+R)** | Places a copy of the selected notes straight after them. Press again to lay more copies end to end. |

These are on the **Edit** menu too. The Piano Roll has its own clipboard:
copying notes doesn't change the system clipboard or the playlist's. Copied
notes stay on it when you show another pattern or row. Pasted and repeated
notes are selected.

## 8-7 Deleting notes

- Select notes and press **Delete** (or **Backspace**).
- *Right-click* a note to delete it.
- **Cut (⌘X)** deletes the selected notes after copying them.

Deleting the notes in the pattern's last bars can make it shorter (see
[7-2-4 Auto-trim](./patterns.md#_7-2-4-auto-trim)).

Each drag, and each delete, is one step to undo with **⌘Z (Ctrl+Z)**.

## 8-8 Velocity and auditioning

Notes you draw have a velocity of 80%. Steps lit in the Pattern Sequencer
have full velocity. Velocity can't be edited yet.

- *Click* a key on the keyboard to hear it while you hold the mouse button.
  *Drag* along the keys to play each in turn.
- Drawing a note, or moving one to a new pitch, plays it briefly.

Notes sound on the row's instrument, straight to the master. On a sample
row, the sample plays at that pitch (see
[7-3-1](./patterns.md#_7-3-1-sample-rows-and-synth-rows)). A muted sample
row's keys still sound; a muted synth row's keys are silent.

## 8-9 Playing the pattern

Click **Play**, or press **Space** while the Piano Roll is the focused
pane's tab. The pattern plays from the playhead, then loops from its start.
A line shows where it is.

- **Stopped:** *click* or *drag* in the ruler to move the playhead. It lands
  on the snap line nearest the mouse, inside the pattern. **Play** starts
  from there.
- **Playing:** *click* or *drag* in the ruler, and playback jumps there when
  you let go.

Playback stops when you leave the Piano Roll and the Pattern Sequencer (see
[7-5 Playing a pattern](./patterns.md#_7-5-playing-a-pattern)).

## 8-10 Scrolling and zooming

- **Scroll:** mouse wheel or two-finger swipe. Up and down scrolls through
  the pitches; left and right (or **Shift** with the wheel) scrolls along
  time. With **Select**, you can also *drag* empty grid.
- **Zoom:** **⌘ (Ctrl)** with the wheel, or pinch on a trackpad, zooms
  along time about the mouse.

These work over the grid, the ruler and the keyboard. A pattern opens with
its notes in the middle of the view, or C3 to C5 when it has none.
