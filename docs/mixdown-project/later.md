# Mixdown — later

- **Live render method.** A "Method: Fast / Live" setting. Live plays the song through the device in real time and captures the master output, feeding the same sink the offline loop feeds. Useful for capturing real-time-only things (knob moves, external gear). Automation or resampling may cover most of this better.
- **Range: loop region.** Mix down only the loop region; the default when the loop is on.
- **ID3 tags** on MP3 (title from the name, artist from preferences).
- **Sample-rate choice** for WAV (44.1/48k regardless of the engine rate).
- **MP3 at low engine rates.** At 22.05/24 kHz LAME quietly caps 320 kbps at about 160. Render MP3s at 44.1k when the engine is below 32k, or show the real bitrate. (Found by the M5 verifier.)
- **LAME/Info tag** at the start of MP3s, so players skip the encoder delay (about 50 ms of silence) and play gaplessly.
- **Progress during the tail.** The bar sits at 100% for up to 10 s while a ringing end renders.
- **Rename fallback race.** On drives that can't hard-link (exFAT, FAT, some SMB shares), a file created under the chosen name in the instant between the check and the rename would be written over.
