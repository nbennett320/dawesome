# writes engine/crates/tape/tests/assets/core_simple.json: the fake deck's
# record side (faketape.core, one channel) on a test signal the rust test
# rebuilds from the same recipe (tests/core.rs, `recipe`), at a few drives.
# run with the tape-model venv, tools/tape-model on sys.path
import json, math, sys
from dataclasses import asdict, replace
import numpy as np

sys.path.insert(0, sys.argv[1] + "/tools/tape-model")
from tape_model.faketape import Deck, core

RATE = 48_000
REF = 10.0 ** (-18.0 / 20.0)

class Lcg:
  def __init__(self):
    self.s = 12345
  # uniform in [-1, 1), from the top 24 bits
  def next(self):
    self.s = (self.s * 1664525 + 1013904223) & 0xFFFFFFFF
    return (self.s >> 8) / 16777216.0 * 2.0 - 1.0

def recipe():
  out = []
  noise = Lcg()
  # a sweep, 20 hz to 20 khz in 1 s, 6 db over 0 vu, 5 ms fades
  n = RATE
  k = math.log(20_000.0 / 20.0)
  for i in range(n):
    t = i / RATE
    ph = 2.0 * math.pi * 20.0 * (math.exp(t * k) - 1.0) / k
    fade = min(1.0, i / 240.0, (n - 1 - i) / 240.0)
    out.append(2.0 * REF * math.sin(ph) * fade)
  out += [0.0] * (RATE // 20)
  # noise bursts at -6, 0, +6 and +12 db over 0 vu, 0.1 s each
  for level in [0.5, 1.0, 2.0, 4.0]:
    for i in range(RATE // 10):
      out.append(level * REF * noise.next())
    out += [0.0] * (RATE // 20)
  # something like music: bass, a pad chord, plucks and hats, 1 s
  for i in range(RATE):
    t = i / RATE
    v = 1.5 * REF * math.sin(2.0 * math.pi * 55.0 * t) * min(1.0, t / 0.02)
    for f in [220.0, 277.18, 329.63]:
      v += 0.5 * REF * math.sin(2.0 * math.pi * f * t)
    beat = int(t / 0.25)
    tp = t - beat * 0.25
    f = [880.0, 1320.0, 660.0, 990.0][beat]
    p = 2.0 * math.pi * f * tp
    v += 2.0 * REF * math.exp(-tp / 0.08) * (math.sin(p) + 0.5 * math.sin(2.0 * p) + 0.25 * math.sin(3.0 * p))
    th = t - int(t / 0.125) * 0.125
    v += REF * math.exp(-th / 0.01) * noise.next()
    out.append(v)
  out += [0.0] * (RATE // 10)
  return np.array(out, dtype=np.float32)

def main():
  repo = sys.argv[1]
  x = recipe()
  # one channel, so no crosstalk (it would add the channel to itself)
  deck = replace(Deck(), crosstalk_db=-400.0)
  every = 12
  cases = []
  for drive in [-6.0, 6.0, 18.0]:
    y = core(x.astype(np.float64)[:, None], replace(deck, drive_db=drive), RATE)[:, 0]
    cases.append({"drive_db": drive, "output": [float("%.7g" % v) for v in y[::every]]})
  at = [0, 1000, 30000, 50000, 60000, 70000, 80000, 100000, 120000]
  spec = {
    "schema": 1,
    "made_by": "faketape.core (tools/tape-model), one channel, crosstalk off, float64, by the script below (run: python script.py <repo>)",
    "rate": RATE,
    "frames": len(x),
    "deck": asdict(deck),
    "check": {"sum_abs": float(np.abs(x.astype(np.float64)).sum()), "at": at, "values": [float(x[i]) for i in at]},
    "every": every,
    "cases": cases,
    "script": open(__file__).read(),
  }
  path = repo + "/engine/crates/tape/tests/assets/core_simple.json"
  with open(path, "w") as f:
    json.dump(spec, f, separators=(",", ":"))
  print(path, len(x), "frames")

main()
