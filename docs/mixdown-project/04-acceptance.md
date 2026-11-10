# Mixdown — acceptance (by hand, in `pnpm tauri dev`)

Automated: `cd engine && cargo test --no-default-features mixdown` (20 tests) and `pnpm exec jest` (mixdownSlice, windowSlice).

- [ ] **Menu.** File › Mixdown… opens a Mixdown tab; choosing it again selects the same tab.
- [ ] **Defaults, saved project.** Name is the project's name; Path is `<project folder>/mixdowns`.
- [ ] **Defaults, unsaved project.** Name is "Untitled"; Path is ~/Music.
- [ ] **Browse.** Browse… picks a folder, and Path shows it.
- [ ] **WAV.** Confirm makes a 16-bit WAV. It plays in QuickTime and sounds like playback, including faders, pan, mute, solo and master, and no metronome.
- [ ] **WAV 24 / 32f.** Both open in QuickTime; Finder's Get Info shows the bit depth.
- [ ] **MP3.** 320 and 128 kbps both play. The 128 kbps file is about 2.5× smaller.
- [ ] **Ringing end.** A song ending on a long sample or pattern hit isn't cut off at the clip's end.
- [ ] **Auto-number.** Confirming twice gives `song.wav` then `song 2.wav`.
- [ ] **Progress and Cancel.** On a long song the bar moves. Cancel returns to the form with "cancelled" shown, and no file or `.part` is left in the folder.
- [ ] **Keep working.** During a long render, play the song, move a fader and edit a clip. Playback is unaffected and the file has the song as it was at Confirm.
- [ ] **Closing the tab.** Close the tab mid-render and reopen it: it still shows progress, then Done.
- [ ] **Show in Finder.** Opens Finder with the file selected.
- [ ] **Typing.** Typing a space or a capital J in the Name field types it, without playing or joining.
- [ ] **Errors.** An empty playlist shows "There's nothing in the playlist to mix down". A blank name disables Confirm.
