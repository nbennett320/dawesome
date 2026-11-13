---
title: Mixer
---

# 10 Mixer

The **Mixer** tab has a channel strip for each playlist track, summed into
the **Master** strip on the right.

Choose **Window ▸ Mixer**, or press **⇧⌘M (Ctrl+Shift+M)**.

## 10-1 Signal flow

Each track's sound goes through, in order:

1. The clips' own settings (clip gain, pitch, fades, clip pan).
2. The track's effects (see [9 Instruments and effects](./plugins.md)).
3. The track's strip: fader, pan, mute and solo.
4. The master's effects, the **Master** fader, and a limiter.

Moving a control is heard straight away, even partway through a clip.

## 10-2 Track strips

![A track strip](./img/mixer-strip.png)
<!-- SCREENSHOT: one track strip, numbered 1–9 -->

1. Name — *Double-click* to rename the track (up to 40 characters). Press
   **Return** (Enter) to keep it, **Escape** to cancel. Clear the name to go
   back to **Track *n***. A track silenced by another track's solo is shown in
   italics.
2. **Pan** — Shows **C** (center), or **L** / **R** and a percentage.
3. Fader scale — +6, 0, -10, -20, -40 dB.
4. Fader — The track's volume.
5. Meter — The track's level.
6. Volume readout — The fader's level in dB. Click it and type a value.
7. **R** and input menu — Arms the track to record, and chooses the input
   channels it records. See [5 Basic recording](./recording.md).
8. **M** / **S** — Mute and solo.
9. **FX** jack — The track's effects (see [10-5](#_10-5-fx-jack)).

### 10-2-1 Fader

The fader goes from -∞ (silent) to +6 dB. The scale gives more room near
0 dB than at the quiet end.

| To | Do this |
| --- | --- |
| Move it | *Drag* the fader cap |
| Jump to a level | *Click* the fader's track |
| Move it finely | Hold **Shift** while dragging |
| Reset to 0 dB | *Double-click* the fader |
| Step it | **↑** / **↓** (0.5 dB), **Shift+↑** / **Shift+↓** (0.1 dB), **Page Up** / **Page Down** (3 dB) |
| Go to the top or bottom | **Home** / **End** |

Keys work after you click the fader.

In the volume readout, type a number in dB (it's kept between -60 and +6),
or `-inf` for silent. Press **Return** to set it, **Escape** to cancel.

### 10-2-2 Pan

*Drag* the **Pan** knob up or down. Hold **Shift** for fine moves.
*Double-click* to center it.

### 10-2-3 Mute and solo

- **M** mutes the track. Its amber light is lit while muted.
- **S** solos the track: only soloed tracks are heard. Its light is lit while
  soloed.

While any track is soloed, the other tracks' mute lights glow dimly: they're
silenced by the solo.

## 10-3 Master strip

![The master strip](./img/mixer-master.png)
<!-- SCREENSHOT: master strip, numbered 1–5 -->

1. **Clip** — Lights red when the mix goes over 0 dB and the limiter rounds
   off its peaks. Click it to clear the light.
2. Fader and meter — The whole mix's volume and level. Works like a track's
   fader.
3. Volume readout — As on a track strip.
4. **Reset** — Puts every fader, pan, mute and solo back to its default.
5. **FX** jack — The master's effects.

The master has no pan, mute or solo.

### 10-3-1 Resetting the mixer

1. Click **Reset**. It changes to **Sure?**.
2. Click it again within 3 seconds.
<!-- VERIFY: whether Reset can be undone -->

## 10-4 Meters

Meters show each strip's level while the song plays, and fall back to silence
after you stop. The color is green up to -10 dB, amber up to 0 dB, and red
above. A thin line holds the latest peak for a moment.
<!-- VERIFY: colors come from the theme's meter low/mid/high; "green/amber/red" is the default theme -->

## 10-5 FX jack

<!-- PROVISIONAL -->

The **FX** jack at the foot of each strip shows the channel's effects:

- Empty — The channel has no effects.
- Plugged, with a number — How many effects are on the channel.
- Blue light — Lit while at least one of them is on.

Click the jack to open the **Effects Rack** on that channel.

*Drag* an effect from the sidebar onto a strip to add it to the end of that
channel's chain. *Drag* an instrument onto a track's strip to add it as a
synth row, with a new pattern of it on that track (see
[9-3-1 Instruments](./plugins.md#_9-3-1-instruments)). The strip is
highlighted while a drop would go on it. See
[9-3 Adding a plugin to a channel](./plugins.md).
