# a made-up, license-free stretch of music for the test file: drums, bass,
# chords and a plucked lead, in sections that build up and drop down, so
# the tape sees quiet and loud, sparse and dense

import numpy as np
from scipy import signal

from .dsp import RATE, bandpass, brickwall, highpass, lowpass, rms

BPM = 100
BEAT = 60.0 / BPM
BAR = 4 * BEAT
BARS = 36

# am, f, c, g: roots for the bass, triads for the pads
CHORDS = [
  (55.00, [220.00, 261.63, 329.63]),
  (43.65, [174.61, 220.00, 261.63]),
  (65.41, [196.00, 261.63, 329.63]),
  (49.00, [196.00, 246.94, 293.66]),
]
# a minor pentatonic, for the lead
SCALE = [440.00, 523.25, 587.33, 659.25, 783.99, 880.00]


def _t(seconds, rate):
  return np.arange(int(seconds * rate)) / rate


def _place(track, x, at, rate):
  i = int(at * rate)
  end = min(len(track), i + len(x))
  if end > i:
    track[i:end] += x[:end - i]


def kick(rate):
  t = _t(0.45, rate)
  freq = 45.0 + 75.0 * np.exp(-t / 0.04)
  body = np.sin(2 * np.pi * np.cumsum(freq) / rate) * np.exp(-t / 0.18)
  click = np.exp(-t / 0.002) * 0.3
  return body + click


def snare(rng, rate):
  t = _t(0.25, rate)
  noise = bandpass(rng.standard_normal(len(t)), 1000, 7000, rate) * np.exp(-t / 0.07)
  tone = np.sin(2 * np.pi * 190 * t) * np.exp(-t / 0.05)
  return 0.9 * noise + 0.5 * tone


def hat(rng, rate, open_=False):
  t = _t(0.3 if open_ else 0.08, rate)
  decay = 0.12 if open_ else 0.02
  return highpass(rng.standard_normal(len(t)), 7000, rate) * np.exp(-t / decay) * 0.35


# a naive saw: aliased, but everything is low-passed after
def saw(freq, t, phase=0.0):
  return 2.0 * ((freq * t + phase) % 1.0) - 1.0


def envelope(n, rate, attack, release):
  env = np.ones(n)
  a = min(n, max(1, int(attack * rate)))
  r = min(n - a, max(1, int(release * rate)))
  env[:a] = np.linspace(0.0, 1.0, a)
  env[n - r:] *= np.linspace(1.0, 0.0, r)
  return env


def bass_note(freq, seconds, rate):
  t = _t(seconds, rate)
  x = saw(freq, t) + 0.5 * saw(freq * 2, t)
  x = lowpass(x, 700, rate, order=4)
  return x * envelope(len(t), rate, 0.005, 0.03) * 0.5


def pad_chord(freqs, seconds, rate, rng):
  t = _t(seconds, rate)
  x = np.zeros(len(t))
  for f in freqs:
    for cents in (-7, 0, 7):
      x += saw(f * 2 ** (cents / 1200), t, rng.random())
  x = lowpass(x, 2500, rate, order=4)
  return x * envelope(len(t), rate, 0.3, 0.4) * 0.08


# karplus-strong: a noise burst through a damped comb, as one iir filter
def pluck(freq, seconds, rate, rng):
  period = int(rate / freq)
  a = np.zeros(period + 2)
  a[0] = 1.0
  a[period] = a[period + 1] = -0.497
  x = np.zeros(int(seconds * rate))
  x[:period] = rng.uniform(-1.0, 1.0, period)
  return signal.lfilter([1.0], a, x) * 0.4


def pan(x, position):
  # constant power, -1 left to 1 right
  angle = (position + 1.0) * np.pi / 4.0
  return np.stack([x * np.cos(angle), x * np.sin(angle)], axis=1)


# which parts play in each bar, and how loud the whole thing is there
def arrangement(bar):
  if bar < 8:
    return {"bass", "hats"}, 0.5
  if bar < 16:
    return {"bass", "hats", "drums"}, 0.8
  if bar < 24:
    return {"bass", "hats", "drums", "pads", "lead"}, 1.0
  if bar < 32:
    return {"pads", "lead"}, 0.6
  return {"bass", "hats", "open_hats", "drums", "pads", "lead"}, 1.0


def render(rng, rate=RATE):
  frames = int(BARS * BAR * rate) + rate
  tracks = {name: np.zeros(frames) for name in ("kick", "snare", "hats", "bass", "lead")}
  pads = np.zeros((frames, 2))
  level = np.zeros(frames)
  k = kick(rate)

  for bar in range(BARS):
    parts, loud = arrangement(bar)
    start = bar * BAR
    level[int(start * rate):int((start + BAR) * rate)] = loud
    root, triad = CHORDS[bar % 4]
    if "drums" in parts:
      for beat in range(4):
        if beat in (0, 2) or (beat == 3 and bar % 2):
          _place(tracks["kick"], k, start + beat * BEAT, rate)
        if beat in (1, 3):
          _place(tracks["snare"], snare(rng, rate), start + beat * BEAT, rate)
    for eighth in range(8):
      at = start + eighth * BEAT / 2
      if "hats" in parts:
        open_ = "open_hats" in parts and eighth % 4 == 2
        _place(tracks["hats"], hat(rng, rate, open_), at, rate)
      if "bass" in parts:
        freq = root * (2 if eighth in (3, 6) else 1)
        _place(tracks["bass"], bass_note(freq, BEAT / 2 * 0.9, rate), at, rate)
      if "lead" in parts and rng.random() < 0.55:
        _place(tracks["lead"], pluck(rng.choice(SCALE), 0.6, rate, rng), at, rate)
    if "pads" in parts:
      left = pad_chord(triad, BAR, rate, rng)
      right = pad_chord(triad, BAR, rate, rng)
      i = int(start * rate)
      pads[i:i + len(left), 0] += left
      pads[i:i + len(right), 1] += right

  # smooth the section levels so they don't step
  level = signal.lfilter([0.0005], [1.0, -0.9995], level)
  mix = (
    pan(tracks["kick"], 0.0)
    + pan(tracks["snare"], 0.05)
    + pan(tracks["hats"], 0.35)
    + pan(tracks["bass"], 0.0)
    + pan(tracks["lead"], -0.3)
    + pads
  )
  return mix * level[:, None]


# the music at about a given rms, peaks softly held near `peak`. the
# low-pass comes last, so the limiting's harmonics don't reach the pilot
def render_at(rng, rms_level, peak, rate=RATE, cutoff=12_000.0):
  x = render(rng, rate)
  x *= rms_level / rms(x)
  x = np.tanh(x / peak) * peak
  return brickwall(x, cutoff, rate)
