---
title: Basic recording
---

# 5 Basic recording

Dawesome records audio from your input device onto playlist tracks. Each
recording makes one take per armed track. When recording stops, each take
is placed on its track as a clip, starting where the recording started.

## 5-1 Choosing the input device

1. Choose **Preferences ▸ Audio Settings…**. The **Audio** tab opens.
2. Choose a device in **Input**. **System default** uses the system's input.
3. Check the input level meter below it. It shows one bar per input
   channel.

The line under the meter shows the input's latency and sample rate. If the
input can't record at the output's rate, the tab says so, and takes are
converted as they play.

**Recording offset (ms)** moves takes earlier or later to line them up with
what you heard while recording. Use it for devices that don't report all
their latency, such as Bluetooth headphones. The default is 150 ms.

## 5-2 Arming tracks

You arm tracks on their mixer strips. Recording needs at least one armed
track.

![A mixer strip's record controls](./img/record-arm.png)
<!-- SCREENSHOT: one mixer strip, the red LED, R button and input menu, numbered 1–3 -->

1. **LED** — Lights red while the track is armed. Blinks while recording.
2. **R** button — Arms the track, or disarms it.
3. **Input menu** — The input channels the track records: a single channel
   (**In 1**, **In 2**, …) or a pair (**In 1+2**, **In 3+4**, …). The default
   is the first two channels, or the only channel on a mono input.

To arm a track:

1. Open the **Mixer** tab (**Window ▸ Mixer**, **⇧⌘M (Ctrl+Shift+M)**).
2. On the track's strip, choose the input channels in the input menu.
3. Click **R**. The LED lights.

You can arm several tracks. Each records its own take.

::: tip NOTE
If the input has fewer channels than the track was armed with (for
example, a project armed on another device), the track records the first
pair, or the first channel, instead.
:::

Armed tracks and the input chosen for each are saved with the project.

## 5-3 Count-in and metronome

- **Count in a bar before recording** (count-in key in the top bar) — When
  lit, recording starts with one bar of clicks. The first click is accented.
  The count-in plays whether or not the metronome is on. Its setting is
  saved with the project.
- **Metronome** (top bar) — When lit, a click plays on each beat during
  playback and recording.

## 5-4 Recording a take

1. Arm one or more tracks (see [5-2](#_5-2-arming-tracks)).
2. Click in the ruler at the top of the playlist to move the playhead to
   where you want to start.
3. Click the record button in the top bar (**Record every armed track from
   the playhead**). Playback starts from the playhead (after the count-in,
   if it's on) and every armed track records.
4. To stop, click the record button again (**Stop recording**). You can
   also stop playback with **Space** or the stop button.

The record button stops playback too, and the playhead stays where it is.
(The stop button goes back to the start.) Each take appears on its track
as a clip.

::: tip NOTE
If no track is armed, the record button's tooltip reads **Record (arm a
track first, with the R key on its mixer strip)**. The "R key" is the **R**
button on the strip, not a keyboard key.
:::

Undo and redo don't work while a take is being recorded.

## 5-5 Recording with loop on

If **Loop** is on in the top bar, what recording does when the loop comes
round is set in **Preferences → Controls → At the loop's end, while recording**:

- **Keep recording, layering each pass** — Recording carries on. Each time
  round is a take of its own, put on the track as soon as it ends, and
  heard on the passes after it. One undo removes the whole recording.
- **Stop recording and playback** — The take and playback both end at the
  loop's end, and the playhead goes back to where recording started.

With a [loop region](./playlist.md#_6-7-2-loop-region), recording loops
inside the region. The first take runs from where you pressed record to the
region's end, and each take after it covers the whole region. If you start
recording before the region, the first take plays into it.

While recording, the loop (and its region) stays where it was when
recording started. The region, the loop end and the Loop button can't be
changed until recording stops.

## 5-6 Where recordings are saved

Takes are WAV files. Each is named after the project, the date (month,
day, year) and a number for that day, starting from 0, for example
`my_song_11092026_0.wav`. Takes recorded together are numbered one after
another.

- **Saved project:** in the `recordings` folder next to the project file.
- **Project never saved:** in a folder of its own in the app's data folder
  (on macOS, `~/Library/Application Support/dawesome/unsaved/`). The takes
  move into the project's `recordings` folder when you save it.

<!-- VERIFY: the unsaved-takes folder on Windows and Linux (the system's data folder + dawesome/unsaved) -->

See [Projects](./projects.md) for the project folder.

## 5-7 Recordings browser

The **Recordings** page of the sidebar lists the project's takes, newest
first, in folders. Click the **Recordings** key on the sidebar rail to show
it.

![The Recordings browser](./img/recordings-browser.png)
<!-- SCREENSHOT: Recordings sidebar page with a folder open and a few takes -->

Each row shows the take's name, length and when it was recorded (the time
today, the day this year, or the date). A dot before the name means the
project uses the take; takes not in use are dimmed. Hold the mouse over a
row to see its full path, when it was recorded, and where it's used.

- **Hear a take:** click it (or select it and press **Return (Enter)**).
- **Use a take:** drag it into the playlist or the Pattern Sequencer.
- **Search:** type in **Search** at the top. Only takes whose names contain
  the text are listed, with their folders. **Esc** clears the search.
- **New folder:** click the new-folder button beside the search field. The
  folder goes in the selected folder, or at the top. You can also
  right-click empty space or a folder and choose **New folder**.
- **Move:** drag a take or folder onto another folder. Hold it over a
  closed folder to open it.
- **Rename:** right-click a take or folder and choose **Rename**. Type the
  name and press **Return (Enter)**; **Esc** leaves it as it was.
- **Delete:** right-click and choose **Move to Trash**, then confirm. A
  take the project uses can't be deleted (remove its clips and Pattern
  Sequencer rows first), nor a folder holding one.

Clips and Pattern Sequencer rows that play a take follow it when it's
moved or renamed. So does the saved project file.

While recording, the browser can't make, move, rename or delete anything.
