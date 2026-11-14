---
title: Playlist
---

# 6 Playlist

The playlist is where you arrange the song. It has tracks, laid out top to
bottom, and a ruler numbered in bars along the top. You place clips on the
tracks: **sample clips** play an audio file, and **pattern clips** play a
pattern from the Pattern Sequencer (see
[7 Patterns and sequencer](./patterns.md)).

The playlist is 128 bars long, in bars of the project's time signature. If
clips already reach further than that, it reaches to the end of the last
clip.

Open it with **Window ▸ Playlist**.

![The playlist](./img/playlist.png)
<!-- SCREENSHOT: playlist with tray, ruler, a few sample and pattern clips, numbered 1–5 -->

1. **Tray** — The mouse tools and snap (see [6-2](#_6-2-tools) and
   [6-6](#_6-6-snap)).
2. **Ruler** — Numbered in bars. Zoomed in, it numbers each beat too: **3.2** is
   bar 3, beat 2. Click or drag here to move the playhead.
3. **Tracks** — Each track has its own mixer strip.
4. **Clips** — Sample clips show their waveform. Pattern clips show a lane
   for each row with notes in the pattern. When every row's notes are plain
   steps (as the Pattern Sequencer shows them), each lit step is a block a
   step wide. Otherwise a sample row's notes are short ticks and a synth
   row's are bars. A pattern clip longer than its pattern shows faint lines
   where the pattern starts over.
5. **New track area** — Below the last track. Drop a clip here to put it on
   a new track.

## 6-1 Moving around

- **Scroll:** mouse wheel or two-finger swipe. Up and down scrolls through
  the tracks; left and right (or **Shift** with the wheel) scrolls along the
  timeline.
- **Zoom:** **⌘ (Ctrl)** with the wheel, or pinch on a trackpad. Zooming in
  stops once the grid is down to 1/16 notes.
- **Pan:** drag on empty track space.
- **Move the playhead:** click or drag in the ruler.

## 6-2 Tools

The tray above the ruler has three mouse tools. Only one is active at a
time; its LED is lit.

| Button | Key | What it does |
| --- | --- | --- |
| **Select (S)** | **S** | Selects, moves and trims clips. |
| **Draw (D)** | **D** | Paints pattern clips along a track. On a clip, it works as **Select**. |
| **Cut (C)** | **C** | The cut tool. Click a clip to split it in two. |

The keys work while the playlist is the focused tab. A tool stays on until
you pick another.

### 6-2-1 Select tool

- **Select a clip:** click it.
- **Add or remove a clip from the selection:** *Shift-click* it.
- **Select with a box (lasso):** *Shift-drag*, starting anywhere except a
  clip's edge. Every clip the box touches is selected, replacing the
  previous selection. A box that touches no clips leaves it as it was.
- **Select every clip:** **⌘A (Ctrl+A)**.
- **Clear the selection:** click empty track space, or press **Esc**.
- **Move clips:** drag a clip. If it's selected, the rest of the selection
  moves with it. Drop it in the new track area to put it on a new track.
- **Open a clip:** *double-click* it. A sample clip opens in the **Sample**
  tab (see [Samples](./samples.md)); a pattern clip opens its pattern in
  the Pattern Sequencer.

### 6-2-2 Cut tool

With the cut tool, a dashed line shows where a click would cut.

1. Press **C**, or click **Cut (C)**.
2. Click a clip where you want to cut it.

The clip becomes two clips. If the clip is selected, every other selected
clip that spans that beat, on any track, is cut too. The cut lands on the snap grid (or the
playhead, see [6-6](#_6-6-snap)). A cut can't be made closer to an edge
than the shortest a clip can be.

With the cut tool, *double-clicking* doesn't open the clip. *Shift-click*
and *Shift-drag* still select.

### 6-2-3 Draw tool

The draw tool paints clips of the pattern you selected or made last (in
the Pattern Sequencer, the pattern browser or the Piano Roll).

1. Press **D**, or click **Draw (D)**.
2. *Drag* along empty space on a track.

Outlines show where the clips will go. When you let go, clips of the pattern
are laid end to end from where the drag started, the last one cut short
where it ended. A *click* without dragging places one whole clip. Both ends
of the drag land on the snap grid (see [6-6](#_6-6-snap)); hold **Option
(Alt)** to turn snapping off.

If no pattern has been selected since the project opened (or the last one
was deleted), the drag makes a new, empty pattern, long enough to hold it in
whole bars, and one clip of it. Open the clip to fill it in.

Each paint is one step to undo, the new pattern with it.

## 6-3 Adding clips

Drag from the sidebar into the playlist. A preview shows where the clip
will land.

- **Samples:** from the **Samples** page (see [Samples](./samples.md)).
- **Recordings:** from the **Recordings** page (see
  [Basic recording](./recording.md)).
- **Clips:** from the **Clips** page (see [Samples](./samples.md)).
- **Patterns:** from the **Patterns** page, or from the Pattern Sequencer
  (see [7-7](./patterns.md#_7-7-placing-patterns-in-the-playlist)). The
  **Draw** tool paints them too ([6-2-3](#_6-2-3-draw-tool)).

Drop on a track, or in the new track area to add a track. With snap on,
the clip's start lands on the grid.

## 6-4 Trimming and stretching

### 6-4-1 Trimming

Move the pointer just inside either end of a clip. A bar marks the edge.
Drag the edge to shorten or lengthen the clip. The sound stays where it is
in time; only the part that plays changes.

- If the clip is selected, the same edge of every selected clip moves by
  the same amount.
- A sample clip's edges can't go past the start or end of its sample.
- A pattern clip's left edge can't go back past where its pattern starts.
  Its right edge can go past the pattern's end, as far as the playlist's
  end: the clip then plays the pattern over and over, and faint lines mark
  where it starts again.
- No edge goes past the playlist's start.
- **Esc** while dragging puts the clips back.

A pattern clip keeps its own length. Making its pattern longer or shorter
doesn't change the clip; drag its edge instead.

### 6-4-2 Stretching

*Shift-drag* a clip's edge to stretch a sample clip, like tape: its pitch
changes so it plays for the new length. The other edge stays where it is.

Stretching works on sample clips only. The pitch can go up to 24
semitones either way.

## 6-5 Editing commands

The commands act on the selected clips while the playlist is the focused
tab. Undo, Redo, Cut, Copy, Paste, Repeat, Select All and Join are also on
the **Edit** menu. Most are on the clip right-click menu (see
[6-8](#_6-8-clip-menu)).

| Command | Keys | What it does |
| --- | --- | --- |
| Undo | **⌘Z (Ctrl+Z)** | Undoes the last change. |
| Redo | **⇧⌘Z (Ctrl+Shift+Z)** | Redoes it. |
| Copy | **⌘C (Ctrl+C)** | Copies the clips to the clipboard. |
| Cut | **⌘X (Ctrl+X)** | Copies the clips to the clipboard and removes them. |
| Paste | **⌘V (Ctrl+V)** | Pastes at the playhead (see below). |
| Repeat | **⌘R (Ctrl+R)** | Places a copy of the clips right after them. |
| Select All | **⌘A (Ctrl+A)** | Selects every clip. |
| Join | **Shift+J** | Makes one clip of the selected clips. |
| Explode | **Shift+E** | Splits pattern clips into a track per row. |
| Delete | **Delete** (or **Backspace**) | Removes the clips. |

See also the [Shortcuts](../shortcuts.md) page.

### 6-5-1 Copy, Cut and Paste

The clipboard belongs to Dawesome; the system clipboard is left alone.

**Paste** (⌘V) places the clips so the earliest starts at the playhead, on
the snap grid, on the tracks they came from. To paste somewhere else,
*right-click* empty track space and choose **Paste**: the clips go at that
point, with the topmost on that track. Nothing is pasted if a clip would
start past the end of the playlist.

### 6-5-2 Repeat

**Repeat** (⌘R) copies the selected clips so the copies start where the
originals end. The copies are selected, so pressing ⌘R again adds the next
copy after them.

### 6-5-3 Join

**Join** (**Shift+J**) makes one clip from two or more clips. The clips
must be on one track and of one kind.

- **Sample clips** that are neighbouring parts of one sample (for example,
  the halves of a cut) are put back together.
- **Other sample clips** are written out as one new WAV file, from the
  earliest start to the latest end, with silence in the gaps. The file goes
  in the project's `clips` folder.
- **Pattern clips** become one clip of a new pattern holding their notes
  one after another, as the clips play them (a looping clip's every time
  round), each row's together. The new pattern is named after the old ones
  (**Verse + Chorus**). The halves of a cut clip that still meet go back to
  one clip of their pattern. The clips must sit on the same sixteenth-note
  grid as the first one, and span no more than 128 bars.

If a join isn't possible, a message says why.

### 6-5-4 Explode

**Explode into tracks** (**Shift+E**) splits each selected pattern clip
into one pattern clip per row with notes in it, sample row or synth row,
each on its own track (so its own mixer strip). Each new clip plays a
pattern of just that row's notes, named after the pattern and the row
(**Verse (Kick)**), with the clip's length, so a looping clip explodes to
looping clips. A synth row's plugin gets a copy on its new track.

- The first row stays on the clip's track, unless other sounds are on it.
  The rest go on the empty tracks below it (new tracks past the last one).
  A row whose clips already sit on a track of their own, from an earlier
  explode, goes back to that track.
- Tracks without a name are named after their rows.
- A pattern an earlier explode made, with exactly that row's notes, is used
  again rather than made twice.
- Sample clips, and clips of a pattern with no notes, are left alone.

### 6-5-5 Deep copy

**Deep copy** (clip menu only) writes each clip out as a WAV file of its
own and switches the clip to play it. The file holds the clip as it plays:
its trim, pitch, fades, pan and gain (not its track's strip or the
master). The clip's settings go back to their defaults so nothing is
applied twice. A pattern clip becomes a sample clip. The file it played
before is left alone.

::: warning CAUTION
A deep copy of a pattern clip holds only its sample rows' notes. Synth
rows' notes aren't written into the file, so they go silent.
:::

The files go in the project's `clips` folder and appear on the **Clips**
page of the sidebar.

::: tip NOTE
**Deep copy** and a **Join** that writes a file need a saved project. If
the project has never been saved, the Save dialog opens first.
:::

## 6-6 Snap

The tray's snap controls set the grid that clips, edges and cuts land on.

- **Snap to grid** — Turns snap on or off. Its LED is lit when on.
- **Snap** menu — The grid size: **1 bar**, **1/2 bar**, **1 beat**,
  **1/2 beat** or **1/4 beat**. The menu is dimmed while snap is off.

When you drag an edge or use the cut tool with playback stopped, the edge
or cut also snaps to the playhead when it's nearer than the grid. Hold
**Option (Alt)** while dragging an edge or cutting to turn snapping off.

## 6-7 Loop

**Loop** in the top bar turns looping on or off. With loop on, playback
jumps back to the start of the loop at its end. Clips still sounding at the
loop's end stop there.

### 6-7-1 Loop end

With no loop region, the loop runs from the start of the playlist. It ends
at the end of the last clip, rounded up to the next bar line (four bars on
an empty playlist). A flag in the ruler marks where it ends.

- *Drag* the flag to move the loop's end. It snaps to the grid, and stays
  where you drop it when clips move.
- *Double-click* the flag to let it follow the last clip again.

### 6-7-2 Loop region

A loop region loops part of the playlist. It shows as a band in the ruler,
with a flag at each end, and the lanes outside it are slightly dimmed.

- *Shift+drag* along the ruler to draw a region. Drawing one turns loop on.
- *Drag* either edge to move it. Dragging an edge past the other swaps
  them. Dropping one edge on the other removes the region.
- *Shift+click*, or *double-click* the band, to remove it.
- A *click* inside the band still moves the playhead.

Edges snap to the grid when snap is on. If you press play with the
playhead before the region, playback plays into it, then loops it. With the
playhead past the region, playback starts at the region's start.

With loop off, the region stays in the ruler, dimmed, and playback plays
through it.

The loop region and a dragged loop end are saved with the project. Undo
doesn't change them.

<!-- VERIFY: loop end flag and loop region gestures, tried in the app -->

## 6-8 Clip menu

*Right-click* a clip to open its menu. If the clip is selected, the
command acts on the whole selection; otherwise only on that clip.

![The clip menu](./img/clip-menu.png)
<!-- SCREENSHOT: right-click menu on a selected pattern clip, all items showing -->

| Item | Shown | What it does |
| --- | --- | --- |
| **Explode into tracks** (⇧E) | Pattern clips | See [6-5-4](#_6-5-4-explode). |
| **Open in Pattern Sequencer** | Pattern clips | Shows the clip's pattern in the Pattern Sequencer. |
| **Cut** (⌘X) | Always | Copies to the clipboard and removes. |
| **Copy** (⌘C) | Always | Copies to the clipboard. |
| **Paste** (⌘V) | Clipboard not empty | Pastes at the playhead. |
| **Repeat** (⌘R) | Always | See [6-5-2](#_6-5-2-repeat). |
| **Split at playhead** | Playhead inside the clip | Cuts the clip (and, if it's selected, the other selected clips) at the playhead. |
| **Join** (⇧J) | Two or more clips | See [6-5-3](#_6-5-3-join). |
| **Deep copy** | Always | See [6-5-5](#_6-5-5-deep-copy). |
| **Delete** (⌫) | Always | Removes the clips. |

*Right-click* empty track space to get a menu with only **Paste**, when
the clipboard isn't empty.

The menu closes when you choose an item, click elsewhere, press **Esc**, or
scroll.
