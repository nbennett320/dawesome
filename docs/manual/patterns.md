---
title: Patterns and sequencer
---

# 7 Patterns and sequencer

A **pattern** is a loop of notes, 1 to 128 bars long. The notes are played
by **rows**, and each row is an instrument:

- A **sample row** plays an audio file, as a simple instrument.
- A **synth row** plays a VST3 instrument plugin (see
  [9 Instruments and effects](./plugins.md)).

Rows are shared by every pattern, and each pattern holds its own notes for
every row. A drum beat is a pattern whose notes are single steps on sample
rows; a melody is notes on a synth row; one pattern can hold both.

You light steps and see every row in the **Pattern Sequencer** tab, draw
notes in the **Piano Roll** (see [8 Piano roll](./piano-roll.md)), file
patterns in the sidebar's pattern browser, and place them in the playlist as
clips.

## 7-1 Opening the Pattern Sequencer

Choose **Window ▸ Pattern Sequencer**, or press **⇧⌘E (Ctrl+Shift+E)**. A
new window already has it open, beside the **Playlist** and the **Mixer**.

![The Pattern Sequencer tab](./img/sequencer.png)
<!-- SCREENSHOT: retake for the Pattern Sequencer: three patterns, two sample rows showing steps, one synth row showing a note preview, the greyed bar past the end, numbered 1–9 -->

1. **Patterns** — One key per pattern. The light above a key is lit while
   that pattern plays.
2. **▾** (**All patterns**) — Every pattern in a list, with a **Search
   patterns** field, to pick one that's scrolled out of sight.
3. **+** (**New pattern**) — Makes a new, empty pattern.
4. **Bars** — The selected pattern's length, on a rolling counter, with
   **▲** and **▼** beside it ([7-2-3](#_7-2-3-pattern-length)).
5. **Used only** — Shows only the rows with notes in the selected pattern
   ([7-3-5](#_7-3-5-used-only)).
6. **Copy** — Makes a copy of the selected pattern, straight after it.
7. **Delete** — Deletes the selected pattern.
8. Rows — One per instrument, in one list: the row's controls, then its
   steps or a picture of its notes ([7-4](#_7-4-steps-and-note-previews)).
9. Greyed bar — One bar of greyed keys past the pattern's end. Click one to
   make the pattern longer ([7-4-3](#_7-4-3-the-greyed-bar)).

When the tab is narrow, **Bars**, **Used only**, **Copy** and **Delete** go
on a line of their own under the pattern keys.

## 7-2 Patterns

### 7-2-1 Making and selecting a pattern

- Click **+** to make a new, empty pattern, one bar long. It is named
  **Sequence** and the lowest number no other pattern uses (**Sequence 1**,
  **Sequence 2**, …), and becomes the selected pattern.
- Click a pattern's key to select it. The Pattern Sequencer and the Piano
  Roll always show the same, selected pattern.
- Click **▾** to pick from every pattern. Type in **Search patterns** to
  narrow the list; **Return (Enter)** picks the first match.
- Click **Copy** to make a variation of the selected pattern. The copy is
  named after it (**Verse copy**).

With no pattern in the project, the tab says **No pattern yet**; click
**Make one**. Adding a row to a project without patterns makes one too.

### 7-2-2 Renaming a pattern

1. *Double-click* the pattern's key.
2. Type a name (up to 24 characters).
3. Press **Return** (Enter) to keep it, or **Escape** to cancel.

Clear the name to show the pattern as **Sequence** and its number. A
pattern's number doesn't change when another pattern is deleted.

### 7-2-3 Pattern length

A pattern is 1 to 128 bars long. Its length changes in three ways:

- **The Bars counter.** Click **▲** for a bar more, or **▼** for a bar
  less. To type a length, *click* the counter's wheels, type 1 to 128, and
  press **Return** (Enter) or click away; **Escape** leaves it as it was.
- **Growing.** Clicking a greyed step ([7-4-3](#_7-4-3-the-greyed-bar)), or
  drawing a note past the end in the Piano Roll, makes the pattern long
  enough to hold it, to the end of that bar.
- **Auto-trim.** Clearing the notes in the last bars shortens the pattern
  again ([7-2-4](#_7-2-4-auto-trim)).

When you shorten a pattern, notes past the new end are kept, but don't
play. Make the pattern longer again to hear them.

The length you set with the counter is also the pattern's **floor**:
auto-trim never makes the pattern shorter than that. Growing doesn't raise
the floor. A pattern made at a length keeps that length as its floor: a new
one from **+** (one bar), the playlist's **Draw** tool or **Join**. **Copy**
and **Explode into tracks** keep the original pattern's floor.



Pattern clips in the playlist keep their own length when their pattern's
length changes (see [7-7](#_7-7-placing-patterns-in-the-playlist)).

### 7-2-4 Auto-trim

When a clear leaves the bars at the end of a pattern empty, the pattern
shortens to the last bar that still has notes. It never goes below its
floor ([7-2-3](#_7-2-3-pattern-length)), nor below one bar.

These clears trim:

- Clicking a lit step off, or dragging across lit steps to clear them.
- Deleting notes in the Piano Roll: **Delete**, **Cut (⌘X)**, or
  *right-clicking* a note.
- Removing a row. Every pattern the row had notes in can shorten.

Moving notes, and cutting them in two with the Cut tool, never trim.

For example: **+** makes a one-bar pattern. Click a greyed step in bar 2,
and the pattern is two bars long. Click that step off again, and it is one
bar long. Set the counter to 4, and it stays four bars long, even empty.

While the pattern plays on its own and the playing step is inside the bars
that would go, the trim waits. It happens once playback leaves those bars
(starts the loop again) or stops. If you put notes back in those bars
first, nothing is trimmed. Playback of the playlist never holds a trim back,
and removing a row trims at once.

::: tip NOTE
A trim is part of the clear that caused it: one **⌘Z (Ctrl+Z)** brings the
notes and the bars back together.
:::

### 7-2-5 Deleting a pattern

1. Select the pattern.
2. Click **Delete**. The key changes to **Sure?** (or **And its clip?** /
   **And *n* clips?** when the pattern is in the playlist).
3. Click it again within 3 seconds.

::: warning CAUTION
Deleting a pattern also deletes all of its clips in the playlist.
:::

## 7-3 Rows

### 7-3-1 Sample rows and synth rows

Every row plays its notes on its instrument.

- **Sample row** — Plays its sample at each note's pitch, against the row's
  **sample pitch**. At the sample pitch, the sample plays as recorded. Each
  semitone higher plays it faster and higher, and each one lower slower and
  lower, as tape does. A note's velocity sets how loud it plays.
  - **One-shot** (the default) — Each note plays the whole sample, however
    long the note is.
  - **Gated** — Each note stops the sample at the note's end, with a short
    fade so it doesn't click.
- **Synth row** — Sends its notes to its instrument plugin.

Every row also has a **step pitch**: the note a clicked step plays
([7-3-4](#_7-3-4-rename-step-pitch-sample-pitch-and-gated)).

### 7-3-2 Adding rows

*Drag* onto the Pattern Sequencer:

- a sample from the sidebar's **Samples** page (or a take from
  **Recordings**, or a file from **Clips**) to add a sample row;
- an instrument from the sidebar's **Instruments** page to add a synth row.

New rows go at the bottom of the list. If the project has no pattern yet,
one is made. The box under the rows says what a drop will do.

Drop a sample on a sample row, or an instrument on a synth row, to
**replace** that row's sample or instrument. The row is outlined and says
**Replace sample with *name*** (or **Replace instrument**). Its notes and
settings stay. Dropped on a row of the other kind, it is added as a new row
instead.

If an instrument can't be loaded, the notice bar says **Couldn’t add the
instrument** and why.

::: tip NOTE
You can also drop an instrument on a track's strip in the **Mixer**. That
adds a synth row, a new pattern, and a clip of it on that track. See
[9-3-1 Instruments](./plugins.md#_9-3-1-instruments).
:::

### 7-3-3 Row controls

![A row of the Pattern Sequencer](./img/sequencer-row.png)
<!-- SCREENSHOT: retake: one Pattern Sequencer row, numbered 1–6 -->

1. Grip (**⋮⋮**) — *Drag* to move the row up or down the list. A line shows
   where it will go.
2. Name — On a sample row, *click* to hear the sample. On a synth row,
   *click* to open the plugin's own interface (see
   [9-5](./plugins.md#_9-5-plugin-interfaces)). A renamed sample row shows
   its sample's name faintly after the new one. A synth row whose plugin
   isn't installed is dimmed and says **Not installed**; its notes and
   settings are kept, and it is silent.
3. **M** — Mutes the row. Its amber light is lit while muted.
4. Volume knob — 0% to 200% of the instrument's own level. *Double-click*
   for 100%.
5. Pan knob — *Double-click* to center.
6. Remove key (**×**) — Removes the row, and its notes from every pattern.

*Right-click* a row's controls for its menu: **Edit in Piano Roll**,
**Rename…**, **Step pitch…**, and on a sample row **Sample pitch…** and
**Gated**, then **Remove**.

::: tip NOTE
Step, row and pattern edits can be undone with **⌘Z (Ctrl+Z)**. Undoing a
removed row brings back its notes in every pattern.
:::

### 7-3-4 Rename, step pitch, sample pitch and gated

Choose these from a row's right-click menu. **Rename…** and the pitches open
a field in place of the name: press **Return** (Enter) or click away to keep
what you typed, or **Escape** to cancel.

- **Edit in Piano Roll** — Opens the Piano Roll on the row's notes in the
  selected pattern (see [8 Piano roll](./piano-roll.md)). It's the way in
  for a row that shows steps, which has no note preview to click.
- **Rename…** — A name for the row (up to 40 characters). Clear it to go
  back to the sample's or instrument's own name.
- **Step pitch…** — The note a clicked step plays. New rows start at
  **C4** (middle C). When you change it, the row's notes in each pattern
  where they all sit at the old step pitch move to the new one, so the row
  keeps showing steps.
- **Sample pitch…** (sample rows) — The note that plays the sample as
  recorded. New rows start at **C4**.
- **Gated** (sample rows) — Switches the row between one-shot and gated
  ([7-3-1](#_7-3-1-sample-rows-and-synth-rows)). The menu shows **✓ Gated**
  while it's on.

Type a pitch as a note name (**C4**, **F#3**, **Eb3**) or a MIDI note number
from 0 to 127 (60 is C4). Anything else changes nothing.

### 7-3-5 Used only

Click **Used only** to hide the rows that have no notes in the selected
pattern. Click it again to show every row. Notes past the pattern's end
count as notes in it.

**Used only** stays on as you select other patterns and open other projects.
It isn't saved with the project.

## 7-4 Steps and note previews

### 7-4-1 Lighting steps

- *Click* a step to light it. Click a lit step to clear it.
- *Drag* across steps to light (or clear) every step you pass over.
- On a step that has the keyboard (press **Tab** to reach it), press
  **Return** (Enter) to switch it on or off.

Each step is a 16th note. A lit step is a note one step long, at the row's
step pitch and full velocity. Every other beat is shaded darker, and a
wider gap marks each bar. While the pattern plays, a column marks the
current step across every row.

### 7-4-2 Steps or a note preview

A row shows steps while its notes in the pattern are plain steps: every note
starts on a step, sits at the row's step pitch and is one step long. (On a
one-shot sample row a note can be any length, since the whole sample plays
anyway.) Velocity doesn't matter.

Otherwise, the row shows a **note preview**: a picture of its notes, as wide
as its steps would be. *Click* the preview to edit the notes in the Piano
Roll. Edit the notes back to plain steps and the row shows steps again.
Changing what a row shows never changes its notes.


### 7-4-3 The greyed bar

After the pattern's last bar, every row shows one more bar of greyed keys.
*Click* a greyed key to make the pattern longer: it grows to the end of that
bar, and the step lights. A pattern 128 bars long has no greyed bar.

## 7-5 Playing a pattern

While the **Pattern Sequencer** or **Piano Roll** tab is the focused pane's
tab, **Space** (or **Play** in the top bar) loops the selected pattern on its
own. Press it again to stop.

A pattern playing on its own plays every row straight to the master, without
the tracks' mixer strips or effects. Muted rows are silent.

The pattern stops when neither tab is focused any more: you select another
tab, click another pane, or close the tab. Going from the Pattern Sequencer
to the Piano Roll keeps it playing.

## 7-6 The pattern browser

Click the patterns key on the sidebar's rail (its tooltip says **Show
patterns**) to show the pattern browser. It lists every pattern in the
project, in one tree of folders.

![The pattern browser](./img/pattern-browser.png)
<!-- SCREENSHOT: retake: Patterns page of the sidebar, a folder open, a pattern with both badges, numbered 1–4 -->

1. **Search patterns** — Shows only patterns whose name contains what you
   type, with the folder each is in.
2. New folder key — Makes a folder (inside the folder you clicked last).
3. Folder — Click to open or close it. The number is how many patterns are
   in it.
4. Pattern — Its light is lit while it plays. After its name come badges
   for what it holds notes for (notes past its end count): a wave (**Plays
   samples**) for sample rows, keys (**Plays instruments**) for synth rows.
   Then its length, and **n×** when it is in the playlist *n* times.

*Click* a pattern to select it and open the **Pattern Sequencer**. *Drag* it
into the playlist to place it, or onto a folder to move it.

### 7-6-1 Folders

Folders only sort patterns; they don't change how anything plays.

- **New folder**: click the new folder key, or *right-click* the list (or a
  folder) and choose **New folder**. Type a name and press **Return**.
- **Rename**: *right-click* the folder and choose **Rename**.
- **Move**: *drag* a pattern or a folder onto a folder. Hold it over a closed
  folder for a moment to open it. Drop on the empty part of the list to move
  it to the top.
- **Delete**: *right-click* the folder and choose **Delete**, then confirm.
  **Delete** is unavailable while any pattern in the folder is in the
  playlist; remove those clips first.

::: warning CAUTION
Deleting a folder deletes its folders and patterns too. The app asks
"Delete "*name*" and the *n* patterns in it? This can't be undone."
:::

## 7-7 Placing patterns in the playlist

1. *Drag* a pattern from the pattern browser, a pattern's key in the Pattern
   Sequencer, or a pattern in its **▾** list.
2. Drop it on a track in the **Playlist**. With snap on, the clip's start
   lands on the grid.

The pattern becomes a clip, as long as the pattern is at that moment. After
that, the clip keeps its own length: making the pattern longer or shorter
doesn't change it. A clip longer than its pattern plays the pattern over and
over (see [6-4-1 Trimming](./playlist.md#_6-4-1-trimming)). Editing the
pattern's notes changes every clip of it.

- The playlist's **Draw** tool (**D**) paints clips of the pattern you
  selected last (see [6-2-3 Draw tool](./playlist.md#_6-2-3-draw-tool)).
- *Double-click* a pattern clip, or *right-click* it and choose **Open in
  Sequencer**, to show its pattern in the Pattern Sequencer.
- **Explode into tracks** (**Shift+E**) splits a pattern clip into one clip
  per row, sample or synth, each on its own track. See
  [6-5-4 Explode](./playlist.md#_6-5-4-explode).

In the playlist, every row of a pattern clip plays through the mixer strip
of the track the clip is on. A synth row gets a copy of its plugin for each
track that plays it, so each clip's notes go through its own track's strip
and effects.

## 7-8 Patterns from older versions

Earlier versions of Dawesome had drum patterns (steps on sample rows) and
note patterns (notes for instruments), each holding one or the other.
Opening a project saved by one turns them into patterns of the kind
described in this chapter:

- The sample rows come first in the row list, then the instruments as synth
  rows.
- Each lit step becomes a one-step note at the row's step pitch (**C4**), at
  full velocity, so the pattern sounds as it did. Each note pattern's notes
  go to their synth row.
- A pattern without a name gets the name it was shown with (**Loop 3**,
  **Sequence 2**), so nothing is renamed. Its floor is its old length, so
  it never trims below what it was.
- The drum and note folder trees become one tree. When a drum folder and a
  note folder of the same name sat in the same place, the note folder gets
  **2** after its name (**Verse 2**).
- Each pattern clip keeps the part of its pattern it played, as a length of
  its own.

::: warning CAUTION
Once saved, the project is in the new format, which older versions of
Dawesome can't open. See
[4-5-3 Projects from older versions](./projects.md#_4-5-3-projects-from-older-versions).
:::
