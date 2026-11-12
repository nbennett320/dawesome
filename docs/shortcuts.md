---
title: Keyboard shortcuts
---

# Keyboard shortcuts

On a Mac, shortcuts use **⌘** only. **⌃** (Control) shortcuts aren't used: ⌃C
doesn't copy. On Windows and Linux, the same shortcuts use **Ctrl**.

Letter keys work by the character they type, so they follow your keyboard
layout. While you are typing in a text field, keys type text instead of
running shortcuts. **⌘R (Ctrl+R)** does nothing there (it never reloads the
window).

## Global and transport

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **Space** | **Space** | Play or pause the playlist. In the **Sequencer** or **Piano Roll** tab, loop the selected pattern (or pause). |
| **⌘N** | **Ctrl+N** | **File ▸ New Project** |
| **⌘O** | **Ctrl+O** | **File ▸ Open…** |
| **⌘S** | **Ctrl+S** | **File ▸ Save** |
| **⇧⌘S** | **Ctrl+Shift+S** | **File ▸ Save As…** |
| **⌘,** | **Ctrl+,** | **Dawesome ▸ Preferences…** (opens the **Preferences** tab) |
| **⇧⌘M** | **Ctrl+Shift+M** | **Window ▸ Mixer** |
| **⇧⌘E** | **Ctrl+Shift+E** | **Window ▸ Sequencer** |

**File ▸ Mixdown…**, **Quit**, and the other **Window** and **Preferences**
menu items have no shortcut.
<!-- VERIFY: whether macOS adds ⌘Q to the custom "Quit" item (build_menu gives it no accelerator) -->

See [Projects](./manual/projects.md) and [Mixdown](./manual/mixdown.md).

## Edit

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **⌘Z** | **Ctrl+Z** | Undo |
| **⇧⌘Z** | **Ctrl+Shift+Z** | Redo |
| **⌘X** | **Ctrl+X** | Cut the selected clips to the clipboard (Playlist) |
| **⌘C** | **Ctrl+C** | Copy the selected clips (Playlist) |
| **⌘V** | **Ctrl+V** | Paste clips at the playhead, snapped (Playlist) |
| **⌘R** | **Ctrl+R** | Repeat: place a copy of the selected clips straight after them (Playlist) |
| **⌘A** | **Ctrl+A** | Select all clips (Playlist) or all notes (Piano Roll) |

In a text field, these act on the text. Dawesome's clip clipboard is its own:
copying clips doesn't change your system clipboard.

<!-- VERIFY: Ctrl+Shift+Z for redo on Windows; Ctrl+Y is not handled -->

## Playlist

These work while the **Playlist** is the selected tab in the focused pane.

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **S** | **S** | Select tool |
| **C** | **C** | Cut tool. Stays on until you press **S**. |
| **Delete** (or **Backspace**) | **Delete** (or **Backspace**) | Delete the selected clips |
| **Shift+J** | **Shift+J** | Join the selected clips (**Edit ▸ Join**) |
| **Shift+E** | **Shift+E** | Explode the selected pattern clips into tracks |
| **Esc** | **Esc** | Clear the selection. While dragging a clip edge, cancel the drag. |

Don't confuse **Shift+E** (explode) with **⇧⌘E (Ctrl+Shift+E)** (open the
Sequencer), or the **cut tool** (**C**) with **Cut (⌘X)**.

See [Playlist](./manual/playlist.md).

## Piano roll

These work while the **Piano Roll** is the selected tab in the focused pane.

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **Delete** (or **Backspace**) | **Delete** (or **Backspace**) | Delete the selected notes |
| **Esc** | **Esc** | Clear the selection |
| **⌘A** | **Ctrl+A** | Select all notes |

See [Piano roll](./manual/piano-roll.md).

## Browsers, menus and text fields

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **Enter** | **Enter** | On a sample or take in a sidebar browser: preview it. On a folder: open or close it. |
| **Esc** | **Esc** | In a browser's **Search** field: clear it |
| **Enter** / **Esc** | **Enter** / **Esc** | While renaming a folder, take, track or pattern: keep the name / cancel |
| **Esc** | **Esc** | Close a clip or browser menu |
| **Esc** | **Esc** | While dragging something from the sidebar: cancel the drag |
| **Enter** | **Enter** | On a step in the **Sequencer**: switch it on or off |

## Knobs and faders

Click a knob or fader first to give it the keyboard.

| Keys | Knob | Fader |
| --- | --- | --- |
| **↑** / **→**, **↓** / **←** | Small step | 0.5 dB |
| **Shift** + arrow | Big step (a tenth of the range) | Fine step, 0.1 dB |
| **Page Up** / **Page Down** | Big step | 3 dB |
| **Home** | Minimum | Maximum (+6 dB) |
| **End** | Maximum | Silent |
| **0** | Centre | — |

::: tip NOTE
**Shift** with the arrow keys makes knobs move *more* but faders move *less*.
:::

## Mouse

| Mac | Windows/Linux | Where | Action |
| --- | --- | --- | --- |
| *Shift-drag* | *Shift-drag* | Knob, fader | Fine adjustment (a quarter of the speed) |
| *Double-click* | *Double-click* | Knob | Back to centre |
| *Double-click* | *Double-click* | Fader | Reset |
| *Click* | *Click* | Fader track | Jump there |
| *Shift-click* | *Shift-click* | Playlist clip | Add to or remove from the selection |
| *Shift-drag* | *Shift-drag* | Playlist | Draw a box to select clips |
| *Drag* an edge | *Drag* an edge | Playlist clip | Trim the clip (and the same edge of the other selected clips) |
| *Shift-drag* an edge | *Shift-drag* an edge | Playlist sample clip | Stretch the clip |
| **Option** while dragging an edge | **Alt** while dragging an edge | Playlist | Ignore snap. Also with the cut tool. |
| *Drag* empty space | *Drag* empty space | Playlist | Scroll |
| *Click* or *drag* | *Click* or *drag* | Playlist ruler | Move the playhead |
| *Double-click* | *Double-click* | Playlist clip | Open it in the **Sample** tab (not with the cut tool) |
| *Right-click* | *Right-click* | Playlist clip | Clip menu |
| *Right-click* | *Right-click* | Empty playlist | **Paste** here (when the clipboard has clips) |
| Wheel | Wheel | Playlist | Scroll tracks; sideways along the timeline |
| *Shift*+wheel | *Shift*+wheel | Playlist | Scroll along the timeline |
| **⌘**+wheel, or pinch | **Ctrl**+wheel | Playlist | Zoom |
| *Click* | *Click* | Piano Roll grid | Draw a note; *drag* right to set its length |
| *Drag* a note | *Drag* a note | Piano Roll | Move it (and the other selected notes) in time and pitch |
| *Drag* a note's end | *Drag* a note's end | Piano Roll | Change its length |
| *Shift-click* | *Shift-click* | Piano Roll note | Add to or remove from the selection |
| *Shift-drag* | *Shift-drag* | Piano Roll, empty grid | Draw a box to select notes |
| *Right-click* | *Right-click* | Piano Roll note | Delete it |
| *Click* and hold | *Click* and hold | Piano Roll keyboard | Hear the pitch |

<!-- VERIFY: on a Mac, Ctrl+wheel also zooms the playlist (the handler accepts ⌘ or Ctrl); pinch zoom is WebKit-only, so Windows/Linux trackpads are untested -->
