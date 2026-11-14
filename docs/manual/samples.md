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

The browser shows the samples folder as a tree of folders and samples.

<!-- VERIFY: where the samples folder is for the user (the engine lists ./assets/ relative to its working folder) -->

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

The **Clips** page of the sidebar lists files Dawesome has written for the
project: the results of **Deep copy** and of a **Join** that writes a file
(see [Playlist](./playlist.md#_6-5-editing-commands)). They are kept in the
`clips` folder next to the project file.

Click the **Clips** key on the sidebar rail to show it. With no clips yet,
it reads **No clips yet. Deep copy or join clips to make some.**

It works like the Recordings browser (see
[Basic recording](./recording.md#_5-7-recordings-browser)): click a clip to
hear it, drag it into the playlist or the Pattern Sequencer, search, make
folders, drag to move, and right-click to **Rename** or **Move to Trash**. A dot
marks the clips the project uses; a clip in use can't be deleted.
