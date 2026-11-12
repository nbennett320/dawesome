---
title: Patterns and sequencer
---

# 7 Patterns and sequencer

A **pattern** is a short loop, 1, 2 or 4 bars long. It holds two things:

- **Steps** — 16th-note hits on the Sequencer's sample rows (a drum machine).
- **Notes** — pitches drawn in the Piano Roll and played by an instrument (see
  [8 Piano roll](./piano-roll.md)).

You make patterns in the **Sequencer** tab, sort them in the sidebar's
pattern browser, and place them in the playlist as clips.

## 7-1 Opening the Sequencer

<!-- PROVISIONAL -->

Choose **Window ▸ Sequencer**, or press **⇧⌘E (Ctrl+Shift+E)**.

![The Sequencer tab](./img/sequencer.png)
<!-- SCREENSHOT: Sequencer tab with three sample rows and two patterns, numbered 1–7 -->

1. **Patterns** — One key per pattern. The light above a key is lit while
   that pattern plays.
2. **+** — Makes a new, empty pattern.
3. **Bars** — **1**, **2**, **4**: the length of the selected pattern.
4. **Piano roll** — Opens the selected pattern in the **Piano Roll** tab.
5. **Copy** — Makes a copy of the selected pattern, straight after it.
6. **Delete** — Deletes the selected pattern.
7. Sample rows — One row per sample, with a key for each step.

## 7-2 Patterns

### 7-2-1 Making and selecting a pattern

- Click **+** to make a new, empty pattern. It is as long as the selected
  pattern (1 bar if none is selected), and becomes the selected pattern.
- Click a pattern's key to select it. The Sequencer and the Piano Roll always
  show the same, selected pattern.
- Click **Copy** to make a variation of the selected pattern.

A new pattern has no name. It is shown by its number. Numbers don't change
when another pattern is deleted.

### 7-2-2 Renaming a pattern

1. *Double-click* the pattern's key.
2. Type a name (up to 24 characters).
3. Press **Return** (Enter) to keep it, or **Escape** to cancel.

Clear the name to show the pattern's number again.

### 7-2-3 Pattern length

Click **1**, **2** or **4** under **Bars**. The pattern's clips in the
playlist get longer or shorter to match.

When you shorten a pattern, notes that start after the new end are removed
and notes that run past it are shortened. Use **Undo** to get them back.
<!-- VERIFY: steps past the new end are hidden but kept (litRows filters them), so lengthening again brings them back? -->

If you change the time signature after setting a pattern's length, no
**Bars** key is lit; the pattern keeps its length in steps.

### 7-2-4 Deleting a pattern

1. Select the pattern.
2. Click **Delete**. The key changes to **Sure?** (or **And its clip?** /
   **And *n* clips?** when the pattern is in the playlist).
3. Click it again within 3 seconds.

::: warning CAUTION
Deleting a pattern also deletes all of its clips in the playlist.
:::

## 7-3 The step sequencer

### 7-3-1 Adding sample rows

Drag a sample from the sidebar's **Samples** page onto the Sequencer. Each
sample becomes a row. If the project has no pattern yet, one is made.

Rows are shared by every pattern; each pattern lights its own steps on them.

### 7-3-2 Lighting steps

- *Click* a step to light it. Click a lit step to clear it.
- *Drag* across steps to light (or clear) every step you pass over.

Each step is a 16th note. Every other beat is shaded darker, and a wider gap
marks each bar. While the pattern plays, the current step is outlined.

### 7-3-3 Row controls

![A sample row](./img/sequencer-row.png)
<!-- SCREENSHOT: one sequencer row, numbered 1–4 -->

1. Sample name — Click to hear the sample.
2. **M** — Mutes the row. Its light is lit while muted.
3. Volume knob — 0% to 200% of the sample's own level. *Double-click* for
   100%.
4. **×** — Removes the row, and its steps from every pattern.

::: tip NOTE
Step, row and pattern edits can be undone with **⌘Z (Ctrl+Z)**.
:::

### 7-3-4 Playing a pattern

<!-- PROVISIONAL -->

While the **Sequencer** or **Piano Roll** tab is the focused pane's tab,
**Space** (or Play in the top bar) loops the selected pattern on its own.
Press it again to stop.

The pattern stops when neither tab is focused any more: you select another
tab, click another pane, or close the tab. Going from the Sequencer to the
Piano Roll keeps it playing.

## 7-4 The note sequencer

<!-- PROVISIONAL -->
<!-- VERIFY: a note sequencer (instrument rows with mini note previews) is in progress on branch vsts but not committed at c2295cb. Write this section once it lands. -->

At present, a pattern's notes are edited in the Piano Roll. See
[8 Piano roll](./piano-roll.md).

## 7-5 The pattern browser

Click the patterns key on the sidebar's rail (its tooltip says **Show
patterns**) to show the pattern browser.

![The pattern browser](./img/pattern-browser.png)
<!-- SCREENSHOT: Patterns page of the sidebar, with a folder open, numbered 1–4 -->

1. **Search patterns** — Shows only patterns whose name contains what you
   type, with the folder each is in.
2. New folder key — Makes a folder (inside the folder you clicked last).
3. Folder — Click to open or close it. The number is how many patterns are
   in it.
4. Pattern — Shows its length, and **n×** when it is in the playlist *n*
   times. The light is lit while it plays.

Click a pattern to select it and open the **Sequencer** tab.

### 7-5-1 Folders

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
<!-- VERIFY: drop on empty list space moves to the top level (useFolderDrop comment says the list itself is the top) -->

## 7-6 Placing patterns in the playlist

1. *Drag* a pattern from the pattern browser, or a pattern's key in the
   Sequencer.
2. Drop it on a track in the **Playlist**.

The pattern becomes a clip. Editing the pattern changes every clip of it.
<!-- VERIFY: whether the drop position snaps to the playlist's snap setting -->

- *Double-click* a pattern clip, or *right-click* it and choose **Open in
  Sequencer**, to show its pattern.
- **Explode into tracks** (**Shift+E**) splits a pattern clip into one clip
  per sample row, each on its own track. See
  [6 Playlist](./playlist.md).

<!-- PROVISIONAL -->
In the playlist, a pattern's notes play on the instrument of the track its
clip is on. A track without an instrument plays only the pattern's steps.
<!-- VERIFY: a pattern clip on a track with no instrument plays its steps and its notes are silent -->
