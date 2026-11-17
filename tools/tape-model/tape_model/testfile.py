# the test file played into the decks. it's always the same: drive is set
# on the deck's input, and the analysis measures it from the drive marker.
# a map of every section goes with it, so the analysis knows what's where
#
# a quiet pilot tone runs under everything from the first sync chirp to the
# last, so the wobble can be tracked through the whole pass. the music,
# noise and clicks are low-passed well under it

from dataclasses import asdict, dataclass, field

import numpy as np

from . import music
from .dsp import RATE, brickwall, fade, from_db, pink, stereo

# raised whenever the file's content changes, so a map says which one it is
# 2: the map lists where sweep harmonics pass the pilot too (same audio)
# 3: anchor noise bursts between the tonal sections
# 4: the drive marker 30 db under 0 vu (at -20 a squashy deck bent it 0.1 db)
VERSION = 4

# the calibration tone's peak: 0 vu on the deck, i.e. 0 db drive
REF_DBFS = -18.0
PILOT_HZ = 15_000.0
PILOT_DBFS = -40.0
# music, noise and clicks stay under this, clear of the pilot
CONTENT_HZ = 12_000.0
# a sweep passing within this of the pilot hides it for a moment
PILOT_GUARD_HZ = 2_000.0
# and so do its harmonics, once tape distorts it: up to this one, while
# within this of the pilot (about its tracking band)
PILOT_GAP_HARMONICS = 5
PILOT_HARMONIC_GUARD_HZ = 600.0

SWEEP_DBFS = (-30.0, -18.0, -10.0)
SWEEP_SECONDS = 10.0
SWEEP_HZ = (20.0, 20_000.0)
# relative to the reference: -30 to +12 db, in 2 db steps
STEP_LEVELS = tuple(range(-30, 13, 2))
STEP_SECONDS = 0.5
BURST_LEVELS = (-24.0, -12.0, -6.0, 0.0, 6.0)
CLICK_LEVELS = (-24.0, -12.0, -6.0, 0.0)
# quiet enough to stay straight even at +18 db drive
DRIVE_MARKER_DB = -30.0
# short noise bursts between tonal sections: tones and sweeps can't be
# placed by their content (a tone looks the same a cycle later), so each
# stretch of them gets one of these nearby to pin it down
ANCHOR_SECONDS = 0.6
ANCHOR_DB = -12.0
FLUTTER_HZ = 3150.0


@dataclass
class Section:
  name: str
  start: float
  end: float
  info: dict = field(default_factory=dict)


@dataclass
class Map:
  rate: int
  seconds: float
  ref_dbfs: float
  pilot: dict
  sections: list
  version: int = VERSION
  seed: int = 1
  # of the written wav file, filled in when it's saved
  sha256: str = ""

  def section(self, name):
    return next(s for s in self.sections if s.name == name)

  def to_json(self):
    return asdict(self)

  @staticmethod
  def from_json(data):
    data = dict(data)
    data["sections"] = [Section(**s) for s in data["sections"]]
    return Map(**data)


class _Builder:
  def __init__(self, rate):
    self.rate = rate
    self.parts = []
    self.sections = []
    self.frames = 0

  def add(self, name, audio, **info):
    audio = stereo(np.asarray(audio, dtype=np.float64))
    if name is not None:
      start = self.frames / self.rate
      self.sections.append(Section(name, start, start + len(audio) / self.rate, info))
    self.parts.append(audio)
    self.frames += len(audio)

  def silence(self, seconds, name=None, **info):
    self.add(name, np.zeros(int(seconds * self.rate)), **info)


def tone(freq, seconds, peak_dbfs, rate=RATE):
  t = np.arange(int(seconds * rate)) / rate
  return fade(np.sin(2 * np.pi * freq * t) * from_db(peak_dbfs), 0.01, rate)


def chirp(rate=RATE):
  # 200 hz to 8 khz in half a second: sharp to cross-correlate
  t = np.arange(int(0.5 * rate)) / rate
  phase = 2 * np.pi * (200.0 * t + 0.5 * (7800.0 / 0.5) * t * t)
  return fade(np.sin(phase) * np.hanning(len(t)) * from_db(REF_DBFS), 0.002, rate)


# a log (farina) sweep
def sweep(peak_dbfs, rate=RATE):
  f1, f2 = SWEEP_HZ
  t = np.arange(int(SWEEP_SECONDS * rate)) / rate
  k = SWEEP_SECONDS / np.log(f2 / f1)
  phase = 2 * np.pi * f1 * k * (np.exp(t / k) - 1.0)
  return fade(np.sin(phase) * from_db(peak_dbfs), 0.05, rate)


# when a sweep starting at `start`, or one of its harmonics, is near the
# pilot: [[start, end], ...] in seconds
def sweep_pilot_gaps(start):
  f1, f2 = SWEEP_HZ
  k = SWEEP_SECONDS / np.log(f2 / f1)
  gaps = []
  for h in range(1, PILOT_GAP_HARMONICS + 1):
    # the harmonic is near the pilot while the sweep is within guard/h of pilot/h
    guard = PILOT_GUARD_HZ if h == 1 else PILOT_HARMONIC_GUARD_HZ
    low = (PILOT_HZ - guard) / h
    high = (PILOT_HZ + guard) / h
    if high <= f1 or low >= f2:
      continue
    a = np.log(max(low, f1) / f1) * k
    b = np.log(min(high, f2) / f1) * k
    gaps.append([start + a, start + b])
  return sorted(gaps)


# one tone stepping up in level, without clicks between steps
def steps(freq, rate=RATE):
  per = int(STEP_SECONDS * rate)
  levels = np.repeat(from_db(REF_DBFS + np.array(STEP_LEVELS, dtype=np.float64)), per)
  smooth = np.convolve(levels, np.hanning(int(0.01 * rate)) / np.hanning(int(0.01 * rate)).sum(), mode="same")
  t = np.arange(len(levels)) / rate
  return fade(np.sin(2 * np.pi * freq * t) * smooth, 0.01, rate)


# pink-noise bursts, peaks held to 3.5x their rms, kept under the pilot.
# a level is the burst's rms against a reference sine's rms
def bursts(rng, rate=RATE):
  out = []
  for level in BURST_LEVELS:
    x = np.clip(pink(rng, rate), -3.5, 3.5)
    x = brickwall(x, CONTENT_HZ, rate)
    x *= from_db(REF_DBFS + level) / np.sqrt(2) / np.sqrt(np.mean(x * x))
    out += [fade(x, 0.005, rate), np.zeros(rate // 2)]
  return np.concatenate(out)


# a short pink-noise burst under the pilot, `ANCHOR_DB` under 0 vu (rms
# against a reference sine's rms)
def anchor(rng, rate=RATE):
  x = brickwall(pink(rng, int(ANCHOR_SECONDS * rate)), CONTENT_HZ, rate)
  x *= from_db(REF_DBFS + ANCHOR_DB) / np.sqrt(2) / np.sqrt(np.mean(x * x))
  return fade(x, 0.01, rate)


# single clicks, four at each level, low-passed under the pilot
def clicks(rate=RATE):
  x = np.zeros(int(len(CLICK_LEVELS) * 4 * 0.25 * rate))
  for i, level in enumerate(CLICK_LEVELS):
    for j in range(4):
      x[int((i * 4 + j) * 0.25 * rate)] = 1.0
  x = brickwall(x, CONTENT_HZ, rate)
  x /= np.max(np.abs(x))
  gains = np.repeat(from_db(REF_DBFS + np.array(CLICK_LEVELS)), int(4 * 0.25 * rate))
  return x * gains[:len(x)]


def build(seed=1, rate=RATE):
  rng = np.random.default_rng(seed)
  b = _Builder(rate)
  b.silence(2.0, "preroll")
  anchors = iter(range(1, 100))

  def add_anchor():
    b.silence(0.5)
    b.add(f"anchor_{next(anchors)}", anchor(rng, rate), relative_db=ANCHOR_DB)
    b.silence(0.3)

  b.add("sync_start", chirp(rate))
  add_anchor()
  b.add("calibration", tone(1000.0, 20.0, REF_DBFS, rate), hz=1000.0, peak_dbfs=REF_DBFS)
  add_anchor()
  b.add("drive_marker", tone(1000.0, 4.0, REF_DBFS + DRIVE_MARKER_DB, rate), hz=1000.0, relative_db=DRIVE_MARKER_DB)
  add_anchor()

  gaps = []
  for level in SWEEP_DBFS:
    start = b.frames / rate
    b.add(f"sweep_{int(level)}", sweep(level, rate), peak_dbfs=level, hz=list(SWEEP_HZ), seconds=SWEEP_SECONDS)
    gaps += sweep_pilot_gaps(start)
    add_anchor()

  for freq in (1000.0, 100.0):
    b.add(f"steps_{int(freq)}", steps(freq, rate), hz=freq, relative_db=list(STEP_LEVELS), step_seconds=STEP_SECONDS)
    add_anchor()

  b.add("bursts", bursts(rng, rate), relative_db=list(BURST_LEVELS), burst_seconds=1.0, gap_seconds=0.5)
  b.silence(1.0)
  b.add("clicks", clicks(rate), relative_db=list(CLICK_LEVELS), per_level=4, every_seconds=0.25)
  b.silence(1.0)
  # music at about 0 vu on average: a reference sine's rms, peaks under -3 dbfs
  b.add("music", music.render_at(rng, from_db(REF_DBFS) / np.sqrt(2), from_db(-3.0), rate, CONTENT_HZ))
  b.silence(1.0)

  t = np.arange(3 * rate) / rate
  one = fade(np.sin(2 * np.pi * 1000.0 * t) * from_db(REF_DBFS), 0.01, rate)
  b.add("left_only", np.stack([one, np.zeros_like(one)], axis=1), hz=1000.0)
  b.silence(0.5)
  b.add("right_only", np.stack([np.zeros_like(one), one], axis=1), hz=1000.0)
  add_anchor()
  b.add("flutter_tone", tone(FLUTTER_HZ, 30.0, REF_DBFS, rate), hz=FLUTTER_HZ)
  b.silence(20.0, "hiss")
  add_anchor()
  b.add("sync_end", chirp(rate))
  b.silence(2.0, "postroll")

  audio = np.concatenate(b.parts)

  # the pilot, from the start of the first chirp to the end of the last
  start = b.sections[1].start
  end = next(s for s in b.sections if s.name == "sync_end").end
  i0, i1 = int(start * rate), int(end * rate)
  t = np.arange(i1 - i0) / rate
  pilot = fade(np.sin(2 * np.pi * PILOT_HZ * t) * from_db(PILOT_DBFS), 0.02, rate)
  audio[i0:i1] += pilot[:, None]

  test_map = Map(
    rate=rate,
    seconds=len(audio) / rate,
    ref_dbfs=REF_DBFS,
    pilot={"hz": PILOT_HZ, "peak_dbfs": PILOT_DBFS, "start": start, "end": end, "gaps": gaps},
    sections=b.sections,
    seed=seed,
  )
  return audio, test_map
