# training pairs from aligned passes: what went into the tape, and what
# came back, on the test file's timeline, only where the alignment is
# exact (quality 2)
#
#   input  = the test file, channel by channel (the drive is applied by
#            the network, from each pass's measured drive)
#   target = the aligned recording / the session's 0 vu gain: the
#            reference pass's drive-marker gain, so 0 vu in at 0 db drive
#            comes back at unity

import json
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import soundfile as sf

from .align import GRID
from .dsp import from_db


@dataclass
class PassData:
  name: str
  drive_db: float
  # (frames, channels): the test file's channel each content channel
  # carries, and the aligned recording, normalized
  inputs: np.ndarray
  targets: np.ndarray
  # per frame, per channel: usable for training
  usable: np.ndarray
  sections: list = field(default_factory=list)


def load_pass(folder, test, test_map, min_quality=2):
  folder = Path(folder)
  m = json.loads((folder / "measure.json").read_text())
  aligned, rate = sf.read(folder / "aligned.wav", dtype="float64", always_2d=True)
  if rate != test_map.rate:
    raise ValueError(f"{folder}: {rate} Hz, the test file is {test_map.rate}")
  z = np.load(folder / "align.npz")
  quality = z["quality"]
  channels = aligned.shape[1]
  # which test-file track each aligned channel carries
  tracks = list(z["channels"]) if "channels" in z.files else list(range(channels))
  n = min(len(aligned), len(test))
  inputs = np.stack([test[:n, min(int(tracks[c]), test.shape[1] - 1)] for c in range(channels)], axis=1)
  targets = np.empty((n, channels))
  usable = np.empty((n, channels), dtype=bool)
  drives = []
  for c in range(channels):
    lv = m["channels"][c]["levels"]
    drive = lv["drive_db"] or 0.0
    drives.append(drive)
    # the session's 0 vu: this pass's marker gain, less its drive
    gain = lv["marker_gain_db"] - drive
    targets[:, c] = aligned[:n, c] / from_db(gain)
    q = np.repeat(quality[:, c], GRID)[:n]
    usable[:, c] = q >= min_quality
  return PassData(folder.name, float(np.mean(drives)), inputs, targets, usable, test_map.sections)


def coverage(p, rate):
  # usable seconds per section, channel 0
  out = {}
  for s in p.sections:
    a, b = int(s.start * rate), int(s.end * rate)
    out[s.name] = float(p.usable[a:b, 0].sum() / rate)
  return out


class Windows:
  # random training windows: `warm` frames to settle the state, then
  # `length` frames that count. a window's whole span must be usable and
  # clear of the held-out spans
  def __init__(self, passes, length, warm, held_out_spans, rate, seed=1):
    self.passes, self.length, self.warm = passes, length, warm
    self.rng = np.random.default_rng(seed)
    self.starts = []
    span = warm + length
    for i, p in enumerate(passes):
      for c in range(p.inputs.shape[1]):
        ok = p.usable[:, c].copy()
        for a, b in held_out_spans:
          ok[int(a * rate):int(b * rate)] = False
        run = np.concatenate([[0], np.cumsum(ok)])
        whole = run[span:] - run[:-span] == span
        for s in np.flatnonzero(whole)[::256]:
          self.starts.append((i, c, int(s)))
    if not self.starts:
      raise ValueError("no usable training windows")

  def __len__(self):
    return len(self.starts)

  def batch(self, size):
    pick = self.rng.integers(0, len(self.starts), size)
    span = self.warm + self.length
    x = np.empty((size, span), dtype=np.float32)
    y = np.empty((size, span), dtype=np.float32)
    d = np.empty((size, span), dtype=np.float32)
    for j, k in enumerate(pick):
      i, c, s = self.starts[k]
      p = self.passes[i]
      x[j] = p.inputs[s:s + span, c]
      y[j] = p.targets[s:s + span, c]
      d[j] = p.drive_db
    return x, d, y
