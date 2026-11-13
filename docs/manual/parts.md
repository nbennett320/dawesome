---
title: Names and functions of parts
---

# 3 Names and functions of parts

## 3-1 The main window

![The main window](./img/main-window.png)
<!-- SCREENSHOT: whole window, default layout, numbered 1–5 -->

1. **Menu bar** — The **Dawesome**, **File**, **Edit**, **Window** and
   **Preferences** menus ([3-2](#_3-2-menu-bar)). On macOS they are at the top
   of the screen.
2. **Top bar** — Transport keys, toggles, tempo and the position counter
   ([3-3](#_3-3-top-bar)).
3. **Notice bar** — Appears under the top bar when there is something to
   report about the project: samples (or plugins) a project couldn't find, a
   save or open that failed, or where a take was saved. *Click* **×** to
   dismiss it.
4. **Sidebar** — A rail of keys down the left edge, and the browser for the
   key that is latched ([3-4](#_3-4-sidebar)).
5. **Workspace** — Tabs in panes that you can split and rearrange
   ([3-5](#_3-5-tabs-and-panes)).

The window's title bar shows the project's name, or **Untitled** before the
project is saved.

## 3-2 Menu bar

### 3-2-1 Dawesome menu

| Item | Key | Function |
| --- | --- | --- |
| **Preferences…** | **⌘, (Ctrl+,)** | Opens the **Preferences** tab. See [2-4](./install.md#_2-4-preferences). |
| **Quit** | — | Quits Dawesome. |

<!-- VERIFY: Quit has no accelerator in build_menu; check whether macOS adds ⌘Q anyway. -->

### 3-2-2 File menu

| Item | Key | Function |
| --- | --- | --- |
| **New Project** | **⌘N (Ctrl+N)** | Starts an empty project. |
| **Open…** | **⌘O (Ctrl+O)** | Opens a project file. |
| **Open Recent** | — | Lists up to eight recently opened or saved projects, and **Clear Recent**. |
| **Save** | **⌘S (Ctrl+S)** | Saves the project. |
| **Save As…** | **⇧⌘S (Ctrl+Shift+S)** | Saves the project under a new name. |
| **Mixdown…** | — | Opens the **Mixdown** tab. See [Mixdown](./mixdown.md). |

See [Projects](./projects.md).

### 3-2-3 Edit menu

| Item | Key | Function |
| --- | --- | --- |
| **Undo** | **⌘Z (Ctrl+Z)** | Undoes the last change to the project. |
| **Redo** | **⇧⌘Z (Ctrl+Shift+Z)** | Redoes what was undone. |
| **Cut** | **⌘X (Ctrl+X)** | Cuts the selected clips (or text) to the clipboard. |
| **Copy** | **⌘C (Ctrl+C)** | Copies the selected clips (or text). |
| **Paste** | **⌘V (Ctrl+V)** | Pastes clips (or text). |
| **Repeat** | **⌘R (Ctrl+R)** | Repeats the selected clips after themselves. |
| **Select All** | **⌘A (Ctrl+A)** | Selects every clip (or all the text). |
| **Join** | — | Joins the selected clips into one. |

In a text field, **Undo**, **Redo**, **Cut**, **Copy**, **Paste** and **Select All** act on the text instead. **Join** has no menu
shortcut; press **Shift+J** with the playlist active. See
[Playlist](./playlist.md).

<!-- VERIFY: Cut, Copy and Paste are system-provided menu items; check their labels and keys on Windows and Linux. -->

### 3-2-4 Window menu

Each item opens its tab, or selects it if it is already open.

| Item | Key |
| --- | --- |
| **Playlist** | — |
| **Sample** | — |
| **Mixer** | **⇧⌘M (Ctrl+Shift+M)** |
| **Transport** | — |
| **Sequencer** | **⇧⌘E (Ctrl+Shift+E)** |
| **Piano Roll** <!-- PROVISIONAL --> | — |
| **Effects Rack** <!-- PROVISIONAL --> | — |

### 3-2-5 Preferences menu

| Item | Function |
| --- | --- |
| **Appearance…** | Opens the **Preferences** tab (the same as **Dawesome ▸ Preferences…**). |
| **Audio Settings…** | Opens the **Audio** tab. See [2-3](./install.md#_2-3-audio-settings). |
| **Controls…** | Opens the **Controls** tab: where plugins open, and the command bar. See [Controls](./install.md#controls). |

## 3-3 Top bar

![The top bar](./img/topbar.png)
<!-- SCREENSHOT: top bar, numbered 1–9 -->

1. **Record** — Records every armed track from the playhead. The key stays
   down while recording; press it again to stop. Arm a track first, with the
   **R** key on its mixer strip. See [Basic recording](./recording.md).
2. **Play** / **Pause** — Starts playback, and pauses it. The key stays down
   while playing. Same as **Space**. When the **Sequencer** (or **Piano
   Roll**) tab is the one you are working in, it loops the selected pattern instead.
3. **Stop** — Stops playback and goes back to the start.
4. **Metronome** — Turns the click on and off. The blue LED above the key is
   lit when it is on.
5. **Count in a bar before recording** — When on, recording starts after one
   bar of clicks.
6. **Loop** — Turns looping on and off.
7. **Tempo** — The tempo in beats per minute, from 1 to 1000. Type a value;
   it is taken up when you stop typing, press **Enter**, or leave the field.
   It can't be changed while recording.
8. **Bar** / **Beat** — The playhead's position in bars and beats, from 1.
9. **Time** — The playhead's position in minutes, seconds and tenths
   (m:ss.t).

::: tip NOTE
**Snap to grid** and the **Snap** length are on the playlist's own tray, not
the top bar. See [Playlist](./playlist.md).
:::

## 3-4 Sidebar

![The sidebar](./img/sidebar.png)
<!-- SCREENSHOT: sidebar rail and samples browser, numbered 1–5 on the rail keys -->

Each key on the rail shows its browser. Press the latched key again to hide
the sidebar. Every browser has a **Search** field at the top; press **Esc**
to clear it.

1. **Samples** — The samples folder as a tree. *Click* a folder to open it,
   *click* a sample to hear it, *drag* a sample into the playlist to add it.
   The **Refresh samples** key lists the folder again. See
   [Samples](./samples.md).
2. **Patterns** — The Sequencer's patterns, in folders. *Click* a pattern to
   show it in the Sequencer; *drag* it into the playlist to place it.
   *Right-click* for **New folder**, **Rename** and **Delete**. See
   [Patterns and sequencer](./patterns.md).
3. **Recordings** — The project's recorded takes. *Click* a take to hear it;
   *drag* it into the playlist, the Sequencer or a folder. *Right-click* for
   **Rename**, **Move to Trash** and **New folder**. A dot marks takes the
   project uses; takes it doesn't use are dimmed. A take in use can't be
   moved to the Trash.
4. **Clips** — Files Dawesome wrote itself, with **Deep copy** and **Join**,
   in the project's `clips` folder. It works like **Recordings**. See
   [Playlist](./playlist.md).
<!-- PROVISIONAL -->
5. **Effects** — The plugins found on your computer, in **Effects** and
   **Instruments** sections, in folders by category. *Drag* an effect onto a
   mixer channel or into the **Effects Rack**; *drag* an instrument onto a
   track in the Mixer. The **Rescan plugins** key looks for plugins again.
   **Couldn’t be read** lists plugins that failed during the scan. See
   [Instruments and effects](./plugins.md).

<!-- VERIFY: the Effects page (and its Instruments section) is VST work not on main, and is being reworked in the working tree; recheck its labels. -->

## 3-5 Tabs and panes

![A pane's tab bar](./img/tab-bar.png)
<!-- SCREENSHOT: a pane's tab bar with two panes side by side, numbered 1–5 -->

1. **Tab** — *Click* to show that view. The blue LED is lit on the selected
   tab of the pane you are working in.
2. **Close** — The **×** on a tab closes it. It shows on the selected tab,
   and on others when you hold the pointer over them.
3. **Split right** — Moves the selected tab into a new pane to the right.
4. **Split down** — Moves the selected tab into a new pane below.
5. **Divider** — *Drag* to resize the panes on either side.

**Split right** and **Split down** need at least two tabs in the pane.

To move a tab, *drag* it:

- onto another tab, to put it before that tab;
- onto the empty space after a pane's tabs, to put it at the end;
- onto the middle of a pane, to move it into that pane;
- onto the edge of a pane, to split the pane on that side.

The tabs are:

| Tab | Shows | Opened from |
| --- | --- | --- |
| **Playlist** | The arrangement: tracks and clips. | Window menu |
| **Sample** | A clip's sample, and how it plays. | Window menu |
| **Mixer** | Every track's channel strip, and the master. | Window menu |
| **Transport** | The tape deck ([3-6](#_3-6-transport-tab)). | Window menu |
| **Sequencer** | Rows of samples and patterns of steps. | Window menu |
| **Piano Roll** <!-- PROVISIONAL --> | A pattern's notes against a keyboard. | Window menu |
| **Effects Rack** <!-- PROVISIONAL --> | One channel's effects, in order. | Window menu |
| **Audio** | Audio settings. | **Preferences ▸ Audio Settings…** |
| **Mixdown** | Export to WAV or MP3. | **File ▸ Mixdown…** |
| **Preferences** | Appearance preferences. | **Dawesome ▸ Preferences…** |
| **Controls** | Where plugins open, and the command bar. | **Preferences ▸ Controls…** |
| *plugin name* <!-- PROVISIONAL --> | A plugin's own interface, when plugins open in a tab. | The plugin's open key |

A new window opens with **Playlist**, **Mixer**, **Transport** and
**Sequencer** in one pane.

<!-- VERIFY: whether the pane layout is kept between launches or saved with projects (initial state is hard-coded in windowSlice). -->

## 3-6 Transport tab

The **Transport** tab is a tape deck. The cassette's reels turn as the song
plays and wind as you rewind or fast forward.

![The Transport tab](./img/transport.png)
<!-- SCREENSHOT: Transport tab with cassette and tape path showing, numbered 1–10 -->

1. **Cassette** — The label shows the project's name (**untitled** before it
   is saved). *Double-click* the cassette, or press **Enter** on it, to rename
   the project: type a name and press **Enter** or *click* away; **Esc**
   cancels. Renaming saves the project; see [4-4](./projects.md).
2. **Rec** — Lights its red LED (blinking while stopped, lit while playing).
   It does not record; use **Record** on the top bar.
3. **Rew** — *Click* to go back to the start of the bar (or the bar before);
   hold to rewind. Rewinding is silent; playback carries on when you let go.
4. **F Fwd** — *Click* to go forward a bar; hold to wind forward.
5. **Play** — Starts playback.
6. **Stop** — Stops playback and goes back to the start.
7. **Pause** — Pauses playback. Stays down while paused; press it again to
   carry on.
8. **Show the cassette** / **Hide the cassette** — Shows or hides the
   cassette, leaving just the keys.
9. **Show the tape over the heads** / **Hide the tape over the heads** —
   Draws the tape pulled out across the deck's heads, under the cassette.
   Only shown while the cassette is.
10. Side — The side letter (**A**) in the label's corner.

<!-- VERIFY: the deck's Rec key is marked in code as "recording isn't connected yet" — confirm before release whether to document it or hide it. -->

The playhead's position is shown on the top bar's counter
([3-3](#_3-3-top-bar)).
