---
title: Samples
---

# 11 Samples

Samples are audio files you place in the playlist or the Pattern
Sequencer. You find them on the sidebar's **Samples** page, and adjust how a
sample clip plays in the **Sample** tab.

## 11-1 Sample browser

Click the **Samples** key on the sidebar rail to show the sample browser.
Click the key again to hide the sidebar.

![The Samples browser](./img/sample-browser.png)
<!-- SCREENSHOT: Samples sidebar page with a folder open -->

The browser shows each folder of samples (see
[11-1-1](#_11-1-1-folders-of-samples)) at the top, as a tree of folders and
samples. **Dawesome**, the samples Dawesome comes with, is first, then your
own folders, then **This project** when the open project has a `samples/`
folder (one that came as a shareable project). Each starts open.

- **Open or close a folder:** click it (or select it and press **Return (Enter)**).
  An empty folder shows **Empty**.
- **Hear a sample:** click it (or select it and press **Return (Enter)**). The
  sample last heard stays highlighted.
- **Use a sample:** drag it into the playlist (see
  [Playlist](./playlist.md#_6-3-adding-clips)) or the Pattern Sequencer,
  where it becomes a sample row (see
  [7-3-2 Adding rows](./patterns.md#_7-3-2-adding-rows)).
- **Search:** type in **Search** at the top. Matching samples are listed by
  name, with the folder each is in. **Esc** clears the search.

::: tip NOTE
Search only looks in folders you've opened so far. If a sample isn't
found, open its folder first.
:::

### 11-1-1 Folders of samples

Choose **Preferences ▸ File Paths…** to open the **File Paths** tab, which
lists the folders the **Samples** browser shows:

- **Dawesome** — The samples Dawesome comes with. Always there, and can't
  be removed.
- Your own folders, in the order you added them, each with its full path.
  A folder that's been moved or deleted shows dimmed, with **Not found**,
  and isn't in the browser until it's back.
- **This project** — The open project's `samples/` folder, if it has one.
  It's listed only while that project is open, and can't be removed.

To add a folder (a sample pack, say), *click* **Add Folder…** and choose it.
A folder that's already listed, inside one that is, or holding one that is,
can't be added; the tab says why.

To take a folder out, *click* its **Remove**. Nothing on disk changes, and
projects using its samples still find them.

Your folders are kept between launches (see
[2-5](./install.md#_2-5-where-settings-are-kept)). A shareable project
names the folders in its `samples/` folder after them (see
[4-8-1](./projects.md#_4-8-1-saving-a-shareable-project)).

## 11-2 Sample tab

The **Sample** tab shows a sample clip from the playlist and adjusts how
that clip plays. It changes the clip, not the file: other clips of the same
sample are not affected.

To open it, *double-click* a sample clip in the playlist, or choose
**Window ▸ Sample**. With no clip chosen, the tab reads **Double-click a
clip in the playlist to inspect it.**

![The Sample tab](./img/sample-tab.png)
<!-- SCREENSHOT: Sample tab with a clip loaded, numbered 1–6 -->

1. **Name** — The clip's file name. Hold the pointer over it to see the
   full path.
2. **Length** — How long the clip plays, in seconds and beats, after
   trimming and pitch.
3. **Audition** (play button) — Plays the clip on its own with its
   settings.
4. **Reset** — Puts every setting back to its default. Dimmed when nothing
   has changed.
5. **Waveform** — The clip as it will play, with its gain and fades. Beat
   lines are numbered from the clip's start. The parts the clip doesn't
   play (after trimming) are dimmed. Drag the handles at the top of the
   envelope to set the fades.
6. **Settings** — A knob and number field for each setting (below).

### 11-2-1 Settings

| Setting | Range | Default | What it does |
| --- | --- | --- | --- |
| **Gain** | −60 to +24 dB | 0 dB | The clip's volume. |
| **Pan** | 100% left to 100% right | Center | Where the clip sits left to right. |
| **Pitch** | −24 to +24 st | 0 st | Raises or lowers the pitch in semitones. The clip also plays faster or slower (12 st up is twice as fast), so its length changes. |
| **Attack** | 0 ms to 10 s, and no longer than the clip | 0 ms | Fade in. |
| **Decay** | 0 ms to 10 s, and no longer than the clip | 0 ms | Fade out. |

To change a setting:

- Drag the knob up or down. Hold **Shift** for fine moves.
- Or type a value in its field and press **Return (Enter)**.
- *Double-click* the setting's label or knob to reset it.

Under the fields for **Pan**, **Pitch**, **Attack** and **Decay**, a short
note describes the value (for example **Center**, the speed for **Pitch**,
or **Fade in**).

Stretching a clip in the playlist (*Shift-drag* an edge) also changes its
pitch (see [Playlist](./playlist.md#_6-4-trimming-and-stretching)).

## 11-3 Clips browser

The **Clips** page of the sidebar lists the project's own audio files:
files you drag in from your computer (see
[11-4](#_11-4-dragging-in-files-from-your-computer)), and the results of
**Deep copy** and of a **Join** that writes a file (see
[Playlist](./playlist.md#_6-5-editing-commands)). They are kept in the
`clips` folder next to the project file.

Click the **Clips** key on the sidebar rail to show it. With no clips yet,
it reads **No clips yet. Drag audio files in from Finder, or deep copy or
join clips.**

It works like the Recordings browser (see
[Basic recording](./recording.md#_5-7-recordings-browser)): click a clip to
hear it, drag it into the playlist or the Pattern Sequencer, search, make
folders, drag to move, and right-click to **Rename** or **Move to Trash**. A dot
marks the clips the project uses; a clip in use can't be deleted.

## 11-4 Dragging in files from your computer

Drag audio files (or folders of them) from Finder onto the window to use
them in the project. Dawesome copies each file into the project's `clips`
folder, so the project keeps working if the original is moved or deleted.
The copy is the file as it was: an MP3 stays an MP3.

Where you let go decides what happens:

- **The playlist:** the files are placed one per track, all starting where
  you let go (snapped to the grid when snap is on), from the track under
  the mouse down. Tracks are added below the last one as needed. While you
  drag, the clips follow the mouse, showing where they'll land. The new
  clips are selected, and one **Undo** takes them all away.
- **The Clips page:** the files are only copied, into the folder you let go
  on (or the top of the list).
- **The Pattern Sequencer:** each file becomes a sample row at the bottom.
  A single file let go on a sample row replaces that row's sample instead.

More than 16 files at once on the playlist asks first: **Place all** puts
them on tracks, **Only copy to Clips** copies them without placing them,
and **Cancel** does nothing.

A dropped folder keeps its shape: its audio files, and those of the
folders inside it, are copied into a folder of the same name in `clips`.
If `clips` already has a folder of that name, the files go into it.

- **The same file twice:** if `clips` already has a file with exactly the
  same contents, under any name, it's used rather than copied again.
- **A different file with the same name:** the copy is numbered, for
  example `kick 1.wav`. Nothing is ever written over.
- **The project's own files:** a file that's already in the project's
  `clips` or `recordings` folder is used where it is.
- **Files that can't be played:** files that aren't audio, or audio that
  can't be read, are left out, and a notice names them. Other files inside
  a dropped folder (a PDF, a readme) are left out without a notice.

**Undo** takes away the clips or rows a drop added, but the copies stay on
the **Clips** page.

In a project that has never been saved, the copies are kept in Dawesome's
application data folder until you save (see
[Projects](./projects.md#_4-5-1-before-you-save)).
