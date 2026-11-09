# Recording

The plan for low-latency recording. The goal is takes that line up with the
timeline to the sample, and an engine that can do software input monitoring
later.

## Where we started

- **Recording was a stub.** `toggle_record_input` never stopped (the flag it
  waited on was never set), the WAV was never finalized, disk writes happened
  inside the input callback behind a `try_lock`, and takes were never placed
  on the timeline.
- **The transport ran on the system clock.** `run_playlist` slept once per
  beat and pushed newly built rodio mixers onto a `DynamicMixer`. The playhead
  was `started.elapsed()`, so nothing knew which output sample was playing which
  timeline position. That is exactly what aligning a recording needs.
- **Old dependencies** (rodio 0.15, cpal 0.13): no control over buffer size,
  and no output-callback timestamps.

## Approach: the engine owns the audio clock

Our own cpal output callback is the clock. The transport position is the
number of frames rendered, stored in an atomic.

- **The scheduler thread** (the old beat loop) stays about one beat ahead of
  playback. It builds clip, pattern and metronome sources as before, tags each
  with an exact start frame, and sends it to the callback through a lock-free
  ring buffer. The callback starts each one on its exact sample.
- **Real-time rules:** the callback never takes a lock and never touches the
  disk. Finished sources are sent back to be dropped off the audio thread.
- **rodio stays** for decoding and the `Source` chain (clip settings, channel
  strips, limiter).
- **The render function runs without a device**, so tests can render frames and
  check that clips, clicks and loop wraps land on the exact frame.

## Decisions

- **Recording target:** tracks are armed (each track gets an arm button and an
  input choice). Record captures every armed track.
- **Sample rate:** the engine follows the device rate. Input and output open at
  the same rate. Samples are resampled on playback as they are now.
- **Alignment:** cpal's capture and playback timestamps map every input sample
  to a timeline frame, which cancels input and output latency. A manual offset
  in settings covers interfaces that report latency wrongly.
- **Loop recording:** each loop pass is a new take. The last pass shows on the
  track, and earlier takes are kept.
- **Count-in:** an optional one-bar toggle.
- **Monitoring:** software monitoring comes later. The engine is built so it
  can be added.
- **Devices:** an Audio Settings tab (built from the `DevicePreferences` stub)
  with input device, output device, buffer size and measured latency.

### Project layout

```
my_project/
├── recordings/
│   ├── my_project_11092026_0.wav
│   └── my_project_11092026_1.wav
├── exports/            (later)
└── my_project.dawesome
```

- New projects and Save As create the folder.
- Takes recorded before the first save go to
  `<app data>/dawesome/unsaved/<YYYYMMDD>-<hex>/`. The app data folder is
  used instead of `/tmp`, because macOS clears unused temp files. On the first
  save, the takes are moved into `recordings/` and renamed after the project.
- Old bare `.dawesome` files keep loading. Their recordings go in a
  `recordings/` folder next to the file.

## Stages

Each stage ships on its own, and the app keeps working after every stage.

0. **Dependency upgrade:** cpal 0.15 and the rodio version that matches it,
   with no change in behaviour.
1. **Sample-clock transport:** an `AudioEngine` with its own output stream,
   voice list and frame clock.
   - The beat loop becomes the scheduler, and the metronome becomes a voice.
   - The engine rate comes from the device.
   - The cursor is computed from the frame clock, minus output latency.
   - Previews share the engine's stream.
2. **Audio Settings tab:** devices and buffer size, saved in the app config
   folder. Changing a setting restarts the engine.
3. **Project folders:** the layout above, including moving unsaved takes on the
   first save.
4. **Input capture:** the input callback copies frames and their timestamp into
   a ring buffer, and a writer thread drains it into a 32-bit float WAV.
   Includes input meters and overflow reporting.
5. **Recording on the timeline:** arm and input choice per track, saved in the
   project. Record starts the transport, the take is aligned using the
   timestamps, and on stop the clips are created on the armed tracks.
6. **Count-in and loop takes.**
7. **Later: software monitoring.** Input goes through a ring buffer and is
   mixed in the output callback through the track strip.
