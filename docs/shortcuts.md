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
| **Space** | **Space** | Play or pause the playlist. In the **Pattern Sequencer** or **Piano Roll** tab, loop the selected pattern (or pause). |
| **⌘N** | **Ctrl+N** | **File ▸ New Project** |
| **⌘O** | **Ctrl+O** | **File ▸ Open…** |
| **⌘S** | **Ctrl+S** | **File ▸ Save** |
| **⇧⌘S** | **Ctrl+Shift+S** | **File ▸ Save As…** |
| **⌘,** | **Ctrl+,** | **Dawesome ▸ Preferences…** (opens the **Preferences** tab) |
| **⇧⌘M** | **Ctrl+Shift+M** | **Window ▸ Mixer** |
| **⇧⌘E** | **Ctrl+Shift+E** | **Window ▸ Pattern Sequencer** |

**File ▸ Mixdown…**, **Quit**, and the other **Window** and **Preferences**
menu items have no shortcut.
<!-- VERIFY: whether macOS adds ⌘Q to the custom "Quit" item (build_menu gives it no accelerator) -->

See [Projects](./manual/projects.md) and [Mixdown](./manual/mixdown.md).

## Command bar

Turn it on in **Preferences ▸ Controls…** (it's off by default). Then type
**:** anywhere but a text field, and the info bar along the bottom of the
window becomes a command line, as in vim. Type a command and press **Enter**.
**Esc**, clicking away, or **Backspace** on an empty line closes it. Case
doesn't matter; an unknown command is named in the info bar.

| Command | Action |
| --- | --- |
| `:playlist`, `:sample`, `:mixer`, `:transport` | Open that tab, or select it where it's already open |
| `:sequencer` (`:seq`, `:notes`) | Open the Pattern Sequencer |
| `:piano-roll` (`:piano`), `:effects-rack` (`:fx`) | Open the Piano Roll or Effects Rack |
| `:audio`, `:mixdown` (`:export`), `:controls` | Open the Audio, Mixdown or Controls tab |
| `:preferences` (`:prefs`, `:appearance`) | Open the Preferences tab |
| `:play` | Play or pause, as **Space** does |
| `:stop` | Stop, and go back to the start |
| `:loop`, `:metronome`, `:countin` | Turn loop, the metronome or count-in on or off |
| `:w` | Save (**File ▸ Save**) |
| `:q`, `:q!` | Close the focused tab |
| `:wq` | Save, then close the focused tab |
| `:qa` | Quit, if nothing is unsaved. Otherwise the info bar says so, and Dawesome stays open. |
| `:qa!` | Quit without saving |
| `:wqa` | Save, then quit. If saving is cancelled, Dawesome stays open. |

## Edit

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **⌘Z** | **Ctrl+Z** | Undo |
| **⇧⌘Z** | **Ctrl+Shift+Z** | Redo |
| **⌘X** | **Ctrl+X** | Cut the selected clips (Playlist) or notes (Piano Roll) to the clipboard |
| **⌘C** | **Ctrl+C** | Copy the selected clips (Playlist) or notes (Piano Roll) |
| **⌘V** | **Ctrl+V** | Paste clips at the playhead, snapped (Playlist), or notes at the mouse (Piano Roll) |
| **⌘R** | **Ctrl+R** | Repeat: place a copy of the selected clips (Playlist) or notes (Piano Roll) straight after them |
| **⌘A** | **Ctrl+A** | Select all clips (Playlist) or all notes (Piano Roll) |

In a text field, these act on the text. Dawesome's clipboards are its own:
copying clips or notes doesn't change your system clipboard.

<!-- VERIFY: Ctrl+Shift+Z for redo on Windows; Ctrl+Y is not handled -->

## Playlist

These work while the **Playlist** is the selected tab in the focused pane.

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **S** | **S** | Select tool |
| **D** | **D** | Draw tool: paint pattern clips |
| **C** | **C** | Cut tool |
| **Delete** (or **Backspace**) | **Delete** (or **Backspace**) | Delete the selected clips |
| **Shift+J** | **Shift+J** | Join the selected clips (**Edit ▸ Join**) |
| **Shift+E** | **Shift+E** | Explode the selected pattern clips into tracks |
| **Esc** | **Esc** | Clear the selection. While dragging a clip edge, cancel the drag. |

Don't confuse **Shift+E** (explode) with **⇧⌘E (Ctrl+Shift+E)** (open the
Pattern Sequencer), or the **cut tool** (**C**) with **Cut (⌘X)**.

See [Playlist](./manual/playlist.md).

## Piano roll

These work while the **Piano Roll** is the selected tab in the focused pane.

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **S** | **S** | Select tool |
| **D** | **D** | Draw tool |
| **C** | **C** | Cut tool |
| **Delete** (or **Backspace**) | **Delete** (or **Backspace**) | Delete the selected notes |
| **Esc** | **Esc** | Clear the selection. While dragging notes, put them back. |
| **⌘A** | **Ctrl+A** | Select all notes |
| **⌘X**, **⌘C**, **⌘V**, **⌘R** | **Ctrl+X**, **Ctrl+C**, **Ctrl+V**, **Ctrl+R** | Cut, copy, paste and repeat notes (see Edit) |
| **Space** | **Space** | Loop the pattern from the playhead, or stop |

See [Piano roll](./manual/piano-roll.md).

## Browsers, menus and text fields

| Mac | Windows/Linux | Action |
| --- | --- | --- |
| **Enter** | **Enter** | On a sample or take in a sidebar browser: preview it. On a folder: open or close it. |
| **Esc** | **Esc** | In a browser's **Search** field: clear it |
| **Enter** / **Esc** | **Enter** / **Esc** | While renaming a folder, take, track or pattern: keep the name / cancel |
| **Esc** | **Esc** | Close a clip or browser menu |
| **Esc** | **Esc** | While dragging something from the sidebar: cancel the drag |
| **Enter** | **Enter** | On a step in the **Pattern Sequencer**: switch it on or off |

## Knobs and faders

Click a knob or fader first to give it the keyboard.

| Keys | Knob | Fader |
| --- | --- | --- |
| **↑** / **→**, **↓** / **←** | Small step | 0.5 dB |
| **Shift** + arrow | Big step (a tenth of the range) | Fine step, 0.1 dB |
| **Page Up** / **Page Down** | Big step | 3 dB |
| **Home** | Minimum | Maximum (+6 dB) |
| **End** | Maximum | Silent |
| **0** | Center | — |

::: tip NOTE
**Shift** with the arrow keys makes knobs move *more* but faders move *less*.
:::

## Mouse

| Mac | Windows/Linux | Where | Action |
| --- | --- | --- | --- |
| *Shift-drag* | *Shift-drag* | Knob, fader | Fine adjustment (a quarter of the speed) |
| *Drag* up/down | *Drag* up/down | Tempo | Faster/slower (*Shift-drag* for fine) |
| *Double-click* | *Double-click* | Tempo | Type a tempo |
| *Double-click* | *Double-click* | Knob | Back to center |
| *Double-click* | *Double-click* | Fader | Reset |
| *Click* | *Click* | Fader track | Jump there |
| *Shift-click* | *Shift-click* | Playlist clip | Add to or remove from the selection |
| *Shift-drag* | *Shift-drag* | Playlist | Draw a box to select clips |
| *Drag* an edge | *Drag* an edge | Playlist clip | Trim the clip (and the same edge of the other selected clips) |
| *Shift-drag* an edge | *Shift-drag* an edge | Playlist sample clip | Stretch the clip |
| **Option** while dragging an edge | **Alt** while dragging an edge | Playlist | Ignore snap. Also with the cut tool. |
| *Drag* empty space | *Drag* empty space | Playlist | Scroll |
| *Click* or *drag* | *Click* or *drag* | Playlist ruler | Move the playhead |
| *Double-click* | *Double-click* | Playlist clip | Open a sample clip in the **Sample** tab, or a pattern clip's pattern in the **Pattern Sequencer** (not with the cut tool) |
| *Drag* empty space | *Drag* empty space | Playlist track, Draw tool | Paint clips of the last selected pattern |
| *Right-click* | *Right-click* | Playlist clip | Clip menu |
| *Right-click* | *Right-click* | Empty playlist | **Paste** here (when the clipboard has clips) |
| Wheel | Wheel | Playlist | Scroll tracks; sideways along the timeline |
| *Shift*+wheel | *Shift*+wheel | Playlist | Scroll along the timeline |
| **⌘**+wheel, or pinch | **Ctrl**+wheel | Playlist | Zoom |
| *Click* | *Click* | Pattern Sequencer step | Light it, or clear it |
| *Drag* | *Drag* | Pattern Sequencer steps | Light (or clear) every step passed over |
| *Click* | *Click* | Pattern Sequencer greyed step | Make the pattern longer, to that bar, and light the step |
| *Click* | *Click* | Pattern Sequencer note preview | Edit the row's notes in the Piano Roll |
| *Drag* the grip | *Drag* the grip | Pattern Sequencer row | Move the row up or down |
| *Right-click* | *Right-click* | Pattern Sequencer row | **Rename…**, **Step pitch…**, **Sample pitch…**, **Gated**, **Remove** |
| *Double-click* | *Double-click* | Pattern key | Rename the pattern |
| *Click* | *Click* | Piano Roll grid, Draw tool | Draw a note; *drag* right to set its length |
| *Drag* a note | *Drag* a note | Piano Roll | Move it (and the other selected notes) in time and pitch |
| *Drag* a note's end | *Drag* a note's end | Piano Roll | Change its length |
| *Shift-click* | *Shift-click* | Piano Roll note | Add to or remove from the selection |
| *Shift-drag* | *Shift-drag* | Piano Roll, empty grid | Draw a box to select notes |
| *Click* | *Click* | Piano Roll note, Cut tool | Cut it in two |
| *Right-click* | *Right-click* | Piano Roll note | Delete it |
| *Click* and hold | *Click* and hold | Piano Roll keyboard | Hear the pitch |
| *Click* or *drag* | *Click* or *drag* | Piano Roll ruler | Move the playhead (Play starts from there) |
| **Option** while dragging | **Alt** while dragging | Piano Roll | Ignore snap |
| **⌘**+wheel, or pinch | **Ctrl**+wheel | Piano Roll | Zoom along time |

<!-- VERIFY: on a Mac, Ctrl+wheel also zooms the playlist (the handler accepts ⌘ or Ctrl); pinch zoom is WebKit-only, so Windows/Linux trackpads are untested -->
