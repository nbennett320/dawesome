# a simulated deck, standing in for the portastudios until there are real
# recordings. it does what a pass through tape does to the test file, with
# known values, and keeps the truth: the analysis has to get them back
#
# not a tape model to ship: just enough of each effect (saturation with
# memory, emphasis, head bump, hf loss, wobble, hiss, dropouts, a capture
# offset) to prove the alignment, measurement and training pipeline

from dataclasses import asdict, dataclass, replace

import numpy as np
from scipy import signal

from .dsp import RATE, apply, bandpass, biquad, from_db, pink, read_at
from .testfile import REF_DBFS


@dataclass
class Deck:
  seed: int = 7
  # how far the input is pushed past 0 vu
  drive_db: float = 0.0

  # the record and playback path
  emphasis_hz: float = 3000.0
  emphasis_db: float = 8.0
  # where the curve bends, in multiples of 0 vu
  headroom: float = 2.8
  asymmetry: float = 0.08
  # a slow squash of hot passages: the curve's memory
  compress: float = 0.15
  compress_seconds: float = 0.03
  head_bump_hz: float = 70.0
  head_bump_db: float = 2.5
  hf_loss_hz: float = 14_000.0
  # how many 12 db/octave stages the hf loss has
  hf_loss_stages: int = 1
  low_cut_hz: float = 25.0
  crosstalk_db: float = -42.0
  # one track on the tape, played to both sides
  mono: bool = False
  # ghost echoes from the layers either side on the reel
  print_through_db: float = -200.0
  print_through_s: float = 1.6

  # the transport, as fractions of the right speed
  speed_error: float = 0.003
  wow_hz: float = 0.7
  wow_depth: float = 1.0e-3
  flutter: tuple = ((7.3, 3.0e-4), (21.0, 1.2e-4))
  drift_depth: float = 4.0e-4
  scrape_depth: float = 4.0e-5
  # sticky tape catching and letting go: sudden dips in speed
  stick_slips_per_minute: float = 0.0

  # noise and wear
  hiss_dbfs: float = -66.0
  # noise that rides on the signal: grit under loud passages
  modulation_noise_db: float = -200.0
  # slow level changes from uneven oxide, as a standard deviation
  level_wander_db: float = 0.0
  dropouts_per_minute: float = 0.0
  dropout_depth_db: tuple = (3.0, 15.0)
  dropout_seconds: tuple = (0.01, 0.08)

  # the capture: when the recording starts, and its level
  start_offset_s: float = 0.73
  out_gain_db: float = -2.3


# a well-used tape: duller, noisier, wobblier, with dropouts
WORN = dict(hf_loss_hz=9_000.0, hiss_dbfs=-60.0, wow_depth=2.0e-3, dropouts_per_minute=20.0)


def worn(deck=None):
  return replace(deck or Deck(), **WORN)


# a cheap 1950s reel from a junk store: acetate gone yellow and sticky, mono,
# dull, thin, hissy, printed through, and catching on the heads
ANCIENT = dict(
  emphasis_db=4.0, headroom=1.6, asymmetry=0.2, compress=0.5,
  head_bump_hz=100.0, head_bump_db=1.5, hf_loss_hz=4_500.0, hf_loss_stages=2,
  low_cut_hz=90.0, mono=True, print_through_db=-34.0,
  speed_error=-0.012, wow_hz=0.45, wow_depth=6.0e-3,
  flutter=((5.1, 1.5e-3), (13.0, 8.0e-4)), drift_depth=3.0e-3, scrape_depth=6.0e-4,
  stick_slips_per_minute=8.0,
  hiss_dbfs=-44.0, modulation_noise_db=-26.0, level_wander_db=1.5,
  dropouts_per_minute=45.0, dropout_depth_db=(6.0, 30.0), dropout_seconds=(0.02, 0.3),
  out_gain_db=-4.0,
)

PRESETS = {"fresh": {}, "worn": WORN, "ancient": ANCIENT}


def preset(name, deck=None):
  return replace(deck or Deck(), **PRESETS[name])


# everything but time: drive, emphasis around the curve, the heads' eq
def core(x, deck, rate=RATE):
  ref = from_db(REF_DBFS)
  x = x * from_db(deck.drive_db)
  x = apply(biquad("highshelf", deck.emphasis_hz, rate, 0.7, deck.emphasis_db), x)
  u = x / ref

  # an rms follower squashing loud passages, so the output depends on the
  # level just before, not only the sample now
  p = np.exp(-1.0 / (deck.compress_seconds * rate))
  env = signal.lfilter([1.0 - p], [1.0, -p], u * u, axis=0)
  u = u / np.sqrt(1.0 + deck.compress * env)

  # an asymmetric soft clip with a gain of 1 for small signals
  a, s = 1.0 / deck.headroom, deck.asymmetry
  y = (np.tanh(a * u + s) - np.tanh(s)) / (a * (1.0 - np.tanh(s) ** 2))

  y = y * ref
  y = apply(biquad("highshelf", deck.emphasis_hz, rate, 0.7, -deck.emphasis_db), y)
  y = apply(biquad("peak", deck.head_bump_hz, rate, 1.2, deck.head_bump_db), y)
  for _ in range(deck.hf_loss_stages):
    y = apply(biquad("lowpass", deck.hf_loss_hz, rate, 0.6), y)
  y = apply(biquad("highpass", deck.low_cut_hz, rate, 0.7), y)
  if deck.mono:
    y = np.repeat(y.mean(axis=1, keepdims=True), y.shape[1], axis=1)
  else:
    y = y + from_db(deck.crosstalk_db) * y[:, ::-1]
  return y + print_through(y, deck, rate)


# the layers either side magnetize each other in storage: a quiet, dull
# copy of the sound a reel's turn early (the pre-echo, louder) and late
def print_through(y, deck, rate=RATE):
  if deck.print_through_db <= -120.0:
    return np.zeros_like(y)
  d = int(deck.print_through_s * rate)
  ghost = bandpass(y, 200.0, 3000.0, rate)
  out = np.zeros_like(y)
  out[:-d] += ghost[d:] * from_db(deck.print_through_db)
  out[d:] += ghost[:-d] * from_db(deck.print_through_db - 6.0)
  return out


# the tape's speed, as a multiple of the right one, for `frames` output frames
def speed(frames, deck, rng, rate=RATE):
  t = np.arange(frames) / rate
  v = np.full(frames, 1.0 + deck.speed_error)

  # wow: its rate wanders a little, as a reel's does as the tape winds
  wow_rate = deck.wow_hz * (1.0 + 0.15 * np.sin(2 * np.pi * 0.05 * t + rng.uniform(0, 2 * np.pi)))
  v += deck.wow_depth * np.sin(2 * np.pi * np.cumsum(wow_rate) / rate + rng.uniform(0, 2 * np.pi))
  for hz, depth in deck.flutter:
    v += depth * np.sin(2 * np.pi * hz * t + rng.uniform(0, 2 * np.pi))

  # drift: slow, random, made at 100 hz and spread over the frames
  slow = signal.lfilter(*signal.butter(2, 0.3, fs=100.0), rng.standard_normal(int(frames / rate * 100) + 2))
  slow *= deck.drift_depth / max(np.std(slow), 1e-12)
  v += np.interp(t, np.arange(len(slow)) / 100.0, slow)

  # scrape flutter: fast and noisy
  scrape = bandpass(rng.standard_normal(frames), 300.0, 3000.0, rate)
  v += scrape * deck.scrape_depth / max(np.std(scrape), 1e-12)

  # stick-slip: the tape catches (speed drops fast), then lets go
  count = rng.poisson(deck.stick_slips_per_minute * frames / rate / 60.0) if deck.stick_slips_per_minute else 0
  for _ in range(count):
    length = int(rng.uniform(0.02, 0.15) * rate)
    at = rng.integers(0, max(1, frames - length))
    depth = rng.uniform(0.02, 0.08)
    shape = np.exp(-np.linspace(0.0, 5.0, length)) * (1.0 - np.exp(-np.linspace(0.0, 40.0, length)))
    v[at:at + length] -= depth * shape / shape.max()
  return v


# dips in level: where the oxide is gone or the tape lifts off the head
def dropouts(frames, deck, rng, rate=RATE):
  gain = np.ones(frames)
  count = rng.poisson(deck.dropouts_per_minute * frames / rate / 60.0)
  for _ in range(count):
    length = int(rng.uniform(*deck.dropout_seconds) * rate)
    at = rng.integers(0, max(1, frames - length))
    depth = 1.0 - from_db(-rng.uniform(*deck.dropout_depth_db))
    gain[at:at + length] -= depth * np.hanning(length)
  return gain


# what years do on top: grit riding on the signal, and level wandering
def wear(y, deck, rng, rate=RATE):
  if deck.modulation_noise_db > -120.0:
    p = np.exp(-1.0 / (0.01 * rate))
    env = signal.lfilter([1.0 - p], [1.0, -p], np.abs(y).mean(axis=1))
    grit = bandpass(rng.standard_normal(len(y)), 500.0, 6000.0, rate)
    grit *= env / max(np.std(grit), 1e-12) * from_db(deck.modulation_noise_db)
    y = y + grit[:, None]
  if deck.level_wander_db > 0.0:
    slow = signal.lfilter(*signal.butter(2, 0.5, fs=100.0), rng.standard_normal(int(len(y) / rate * 100) + 2))
    slow *= deck.level_wander_db / max(np.std(slow), 1e-12)
    y = y * from_db(np.interp(np.arange(len(y)) / rate, np.arange(len(slow)) / 100.0, slow))[:, None]
  return y


# one pass of `audio` through the fake deck: what comes back, and the truth.
# truth["pos"][k] is where in `audio` the output's frame offset + k came from
def run(audio, deck=None, rate=RATE):
  deck = deck or Deck()
  rng = np.random.default_rng(deck.seed)
  played = core(audio, deck, rate)

  frames = int(len(audio) / (1.0 + deck.speed_error)) + rate
  v = speed(frames, deck, rng, rate)
  pos = np.concatenate([[0.0], np.cumsum(v)[:-1]])
  pos = pos[pos < len(audio) - 1]
  wobbled = read_at(played, pos)
  wobbled *= dropouts(len(wobbled), deck, rng, rate)[:, None]
  wobbled = wear(wobbled, deck, rng, rate)

  offset = int(deck.start_offset_s * rate)
  out = np.concatenate([np.zeros((offset, audio.shape[1])), wobbled])
  sides = 1 if deck.mono else out.shape[1]
  hiss = np.stack([pink(rng, len(out)) for _ in range(sides)], axis=1)
  out += hiss * from_db(deck.hiss_dbfs)
  out *= from_db(deck.out_gain_db)

  truth = {"deck": asdict(deck), "rate": rate, "offset": offset, "pos": pos}
  return np.clip(out, -1.0, 1.0), truth
