# 01b — Follow-up questions and answers

Asked and answered in chat on 2026-11-10, after batch 1 (`01-design-questions.md`, answers in `01-answers.md`). Each row gives the question, the options offered, and your answer. Text in quotes is your own wording.

## Working rules

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F1 | Do my commits carry Claude Code's trailer lines (Co-Authored-By, session link)? | No trailers / keep the trailers | **No trailers.** One short lowercase line, like your own commits. |

## 64-bar playlist (from Q7)

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F2 | What does Loop cover now? | Up to the last clip / the whole 64 bars / always the first 4 bars | **Up to the last clip:** from the start to the end of the last clip, rounded up to the next bar. An empty playlist loops 4 bars. |
| F3 | What does the Playlist show when it opens? | First 4 bars / first 8 bars / whole playlist | **The first 4 bars.** You can scroll right and zoom out to the whole 64 bars. |
| F4 | Grid when zoomed out | Confirm / change | **Confirmed:** beat lines hide when closer than about 8 px, bar lines stay, ruler numbers are thinned so they never overlap. |
| F39 | Is the loop's end shown on the ruler? | A small marker when Loop is on / no marker | **A small marker** on the ruler at the loop's end, in an existing theme colour, only while Loop is on. |
| F40 | Looping while recording | Confirm / change | **Confirmed:** the loop's end is fixed where it was when the recording started. |

## Tempo (from Q8)

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F5 | Tempo changed while playing | Carries on at the new tempo / applies at the next Play | **Carries on at the new tempo:** playback restarts from the beat it had reached, with a very short fade. |
| F6 | Playhead on a tempo change | Confirm / change | **Confirmed:** the playhead keeps its beat. |
| F12 | Tempo changed while recording | Tempo locked while recording / change ends the take | **Tempo is locked while recording.** |

## Whole-project undo (from Q10)

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F7 | A recording finishes and places clips. What happens to the history? | History kept / history cleared | **History kept.** The recorded clips stay as if they had always been there. |
| F8 | When undo and redo do nothing | Confirm / change | **Confirmed:** while a recording is in progress and while a mouse drag is in progress. |
| F9 | File operations in the Recordings and Clips pages | Confirm / change | **Confirmed:** never undo steps. |
| F10 | Edit menu contents and Undo/Redo labels | Plain labels / labels name the action | **Plain labels:** Undo, Redo; divider; Cut, Copy, Paste, Repeat, Select All; divider; Join. |
| F11 | Redo after a new edit | Confirm / change | **Confirmed:** undone steps can no longer be redone once a new edit is made. |

## Pattern clips (from Q26)

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F13 | Which hits play in a trimmed pattern clip? | Hits that start inside, ringing out / hits cut at the right edge | **Hits that start inside the window play and ring out naturally.** |
| F14 | Right edge dragged past the pattern's end | Stops at the pattern's end / the pattern repeats | **Stops at the pattern's end.** |
| F15 | Shortest pattern clip | One step / 10 ms | **One sixteenth-note step.** |
| F16 | The pattern's length is changed after a clip of it was trimmed | Window kept, clamped if needed / trim discarded | **Window kept, clamped if needed.** Untrimmed clips keep following the pattern. |
| F17 | Detail of the clamping | Confirm / change | **Confirmed:** the clip remembers its original window, so lengthening the pattern brings it back. A window entirely past the pattern's end leaves a one-step clip that plays nothing. |
| F18 | How a trimmed pattern clip is drawn | Confirm / change | **Confirmed:** only the steps in its window, same step width, same edge notches as sample clips. |
| F19 | Cutting a pattern clip | Confirm / change | **Confirmed:** two clips linked to the same pattern, each a window on it. Rejoining halves that still line up gives one clip of the same pattern and no new pattern. |
| F20 | Length of a pattern clip's deep-copied file | Exactly the clip's length / long enough for the tails | **Exactly the clip's length;** tails past the end fade out over 10 ms. |
| F21 | Rest of a pattern clip's deep copy | Confirm / change | **Confirmed:** rendered as it plays at that moment (steps, row volumes, muted rows silent, current tempo); named `<pattern name>_copy_<n>.wav`; the clip becomes an ordinary sample clip; undo turns it back. |
| F22 | Joined pattern is not 1, 2 or 4 bars | Any whole number of bars / only 1, 2 or 4 / up to 8 | **Any whole number of bars.** The Sequencer's 1 / 2 / 4 keys stay; on a longer pattern none is lit. |
| F23 | Turning the span into a pattern | Rounded up to a whole bar / exact span | **Rounded up to a whole bar,** with the new clip trimmed to the exact span so nothing moves. |
| F24 | Clips not on the same sixteenth-note grid | Refused with a message / moved to the nearest step | **Refused with a message.** |
| F25 | Joined pattern's name | "first + second" / "first (joined)" / unnamed | **"first + second"**, numbers for unnamed patterns, "verse + 3 more" beyond three names. |
| F26 | Remaining pattern-join rules | Confirm / change | **Confirmed:** one track only; overlapping steps merged; new pattern placed after the first source pattern in the same folder; originals untouched; undo removes the new pattern and restores the clips. |

## Clips sidebar page (from Q116)

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F27 | Behaviour of the Clips page | Confirm / change | **Confirmed:** exactly like the Recordings page today, pointed at `clips/` (search, preview, drag to Playlist or Sequencer, "in use" marker, rename with clips following, delete to Trash refused while in use, folders). Joined files appear there too. |
| F28 | The page's key on the sidebar rail | I draw a new icon / you supply it | **I draw the icon** in the existing line style and show it to you for approval. |

## Trim and stretch details

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F29 | What the trim notch looks like | Triangle in the label band / notch cut into the edge / dashed edge | **A small solid triangle in the label band** at the trimmed edge, pointing inward, in the clip's border colour. |
| F30 | Sample tab for a trimmed clip | Confirm / change | **Confirmed:** whole file drawn, played part highlighted, rest dimmed; fades and handles inside the highlight; Length shows the played length; the highlight cannot be dragged. |
| F32 | Playhead snap for trim and stretch edges | Confirm / change | **Confirmed:** the edge goes to the playhead when the cursor is nearer to it than to either neighbouring grid line; only with Snap on and Option not held; only while stopped or paused. |
| F33 | Multi-selection trim | Confirm / change | **Confirmed:** same edge, same amount of time; only the grabbed clip's edge snaps; each clip stops at its own limits; pattern clips trim along. |
| F34 | Multi-selection stretch | Confirm / change | **Confirmed:** same ratio for every selected sample clip, each keeping its own opposite edge and its own ±24 semitone limit; pattern clips are left alone. |

## Cut details

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F31 | "Split at playhead" while playing | Splits where the playhead is / not offered while playing | **Splits where the playhead is** at the moment you choose it, unsnapped. Left out when the playhead does not cross the clip. |
| F35 | How exact "halves equal the whole" must be | Identical at any pitch and rate / identical only without resampling | **Identical at any pitch and any file rate,** outside fades cut short by Q95. |

## Playlist tray, menus, messages

| # | Question | Options offered | Your answer |
|---|---|---|---|
| F36 | Playlist tray layout | Confirm / change | **Confirmed:** Select and Cut keys, a gap, then the Snap key and snap-value drop-down, in the `ft-tray` style. The topbar keeps Metronome, Count-in and Loop. |
| F37 | Where short messages appear | Confirm / change | **Confirmed:** the existing notice banner; info style for refusals, error style for failures. |
| F38 | Empty clipboard | Confirm / show Paste greyed out | **Confirmed:** Paste is left out of a clip's menu, right-clicking empty space opens no menu, Cmd+V does nothing. |

## Last gaps

| # | Question | Options offered | Your answer |
|---|---|---|---|
| G1 | Paste while stopped or paused with the playhead off the grid | Snapped when Snap is on / exactly at the playhead | **Snapped when Snap is on,** exact when Snap is off: one rule in every state. |
| G2 | Deep copy on a selection of several clips | One file per clip / identical clips share a file | **One file per clip.** One undo step; if any file fails, none of the clips is changed. |
| H1 | What is selected after a Join | The joined clip / nothing | **The joined clip.** |
| H2 | Pasting a pattern clip whose pattern was deleted since the copy | That clip is skipped / whole paste refused | **That clip is skipped;** the rest is pasted. |
| H3 | Does New Project or Open reset the Playlist view? | Reset to the first 4 bars / leave the view alone | **Reset to the first 4 bars,** scrolled to the start. |
