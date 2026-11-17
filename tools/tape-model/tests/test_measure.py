# measurement, against the fake deck's known values. criteria m1-m9 are in
# context/tape-project/02-plan.md

from dataclasses import replace

import numpy as np
import pytest
from scipy import signal

from tape_model import align, faketape, measure, testfile
from tape_model.dsp import RATE, biquad, from_db


def small_signal_db(deck, hz):
  # the deck's linear response: head bump, hf loss, low cut, crosstalk (the
  # test file's sweeps are the same on both sides)
  w = 2 * np.pi * np.asarray(hz) / RATE
  h = np.ones(len(w), dtype=complex)
  stages = [biquad("peak", deck.head_bump_hz, RATE, 1.2, deck.head_bump_db)]
  stages += [biquad("lowpass", deck.hf_loss_hz, RATE, 0.6)] * deck.hf_loss_stages
  stages += [biquad("highpass", deck.low_cut_hz, RATE, 0.7)]
  for b, a in stages:
    h *= signal.freqz(b, a, worN=w)[1]
  return 20 * np.log10(np.abs(h) * (1 + from_db(deck.crosstalk_db)))


class _Straight:
  # a "recording" already on the test file's timeline (the core's output)
  def __init__(self, x):
    self.rec = x

  def __call__(self, t0, t1):
    return self.rec[int(t0 * RATE):int(t1 * RATE)]

  def frames(self, t0, t1):
    return int(t0 * RATE), int(t1 * RATE)

  def speed(self, t0, t1):
    return 1.0


@pytest.mark.slow
def test_m1_drive_against_the_reference_pass(passes):
  # the fake deck's squash (driven by the pre-emphasized pilot) bends even
  # a quiet marker by ~0.1 db at +6 db: the measurement must match what the
  # deck really did to it, which is what the network will be told
  zero, six = passes("fresh"), passes("fresh+6")
  m = six.measured(reference=zero.measured())
  test, test_map = zero.test, zero.map
  s = test_map.section("drive_marker")
  levels = []
  for p in (zero, six):
    y = faketape.core(test, p.deck)[:, 0]
    levels.append(measure._tone_db(y[int((s.start + 0.5) * RATE):int((s.end - 0.5) * RATE)], 1000.0, RATE))
  effective = levels[1] - levels[0]
  for ch in m["channels"]:
    assert abs(ch["levels"]["drive_db"] - effective) <= 0.02
    assert abs(ch["levels"]["drive_db"] - 6.0) <= 0.15


@pytest.mark.slow
def test_m2_response(passes):
  p = passes("fresh")
  r = p.measured()["channels"][0]["response"]
  hz, db = np.array(r["hz"]), np.array(r["db"])
  want = small_signal_db(p.deck, hz)
  want -= want[np.argmin(np.abs(hz - 1000.0))]
  sel = (hz >= 31.5) & (hz <= 12_500.0)
  assert np.max(np.abs(db[sel] - want[sel])) <= 0.5


@pytest.mark.slow
def test_m3_harmonics_match_the_core(passes):
  p = passes("fresh")
  ch = p.measured()["channels"][0]
  core = faketape.core(p.test, p.deck)[:, 0] * from_db(p.deck.out_gain_db)
  for got in ch["harmonics"]:
    name = "steps_1000" if got["hz"] == 1000.0 else "steps_100"
    want = measure.harmonics(_Straight(core), p.test, p.map, RATE, 0, ch["levels"]["marker_gain_db"], name, 0.0)
    compared = 0
    for g, w in zip(got["steps"], want["steps"]):
      for k in (0, 1):
        if g["harmonics_db"][k] is not None and w["harmonics_db"][k] is not None and w["harmonics_db"][k] > -70:
          assert abs(g["harmonics_db"][k] - w["harmonics_db"][k]) <= 1.0
          compared += 1
    assert compared >= 10
    assert got["steps"][0]["thd_pct"] < 0.1


@pytest.mark.slow
def test_m4_hiss(passes):
  p = passes("fresh")
  got = p.measured()["channels"][0]["hiss"]["db_re_0vu"]
  # the fake's pink hiss spreads its power as 1/f over every bin of the
  # pass: the share in the measured band (pilot notched out)
  n = len(p.rec)
  k = np.arange(1, n // 2 + 1)
  f = k * RATE / n
  band = (f >= 20) & (f < 20_000) & ~(np.abs(f - p.map.pilot["hz"] * p.alignment.sync.speed) <= measure.PILOT_NOTCH_HZ)
  share_db = 10 * np.log10(np.sum(1 / k[band]) / np.sum(1 / k))
  want = p.deck.hiss_dbfs + share_db - (testfile.REF_DBFS - 3.0103) - small_signal_db(p.deck, [1000.0])[0]
  assert abs(got - want) <= 0.5


def _truth_speed_bands(p, cut):
  s = p.map.section("flutter_tone")
  a, b = cut.frames(s.start + 0.5, s.end - 0.5)
  pos, off = p.truth["pos"], p.truth["offset"]
  v = np.diff(pos[a - off:b - off + 1])
  v = v[: len(v) // align.GRID * align.GRID].reshape(-1, align.GRID).mean(axis=1)
  dev = v / v.mean() - 1
  rate = RATE / align.GRID
  edge = int(rate)

  def band(lo, hi):
    sos = signal.butter(2, [lo, hi], btype="bandpass", fs=rate, output="sos")
    return np.sqrt(np.mean(signal.sosfiltfilt(sos, dev)[edge:-edge] ** 2))

  return band(*measure.WOW_HZ), band(*measure.FLUTTER_HZ), float(np.mean(np.diff(pos[a - off:b - off + 1])))


@pytest.mark.slow
def test_m5_wow_and_flutter(passes):
  p = passes("fresh")
  wf = p.measured()["channels"][0]["wow_flutter"]
  wow, flutter, _ = _truth_speed_bands(p, measure._Cutter(p.alignment.recording[:, 0], p.alignment.tracks[0], RATE))
  assert abs(wf["wow_rms"] / wow - 1) <= 0.05
  assert abs(wf["flutter_rms"] / flutter - 1) <= 0.05
  for hz, depth in p.deck.flutter:
    match = [q for q in wf["peaks"] if abs(q["hz"] - hz) <= 0.1]
    assert match and abs(match[0]["depth"] / depth - 1) <= 0.15


@pytest.mark.slow
def test_m6_crosstalk_and_mono(passes):
  ct = passes("fresh").measured()["crosstalk"]
  assert abs(ct["left_into_right"] + 42.0) <= 1.0 and abs(ct["right_into_left"] + 42.0) <= 1.0
  assert not ct["mono"]
  assert passes("ancient").measured()["crosstalk"]["mono"]


@pytest.mark.slow
def test_m7_dropouts(passes):
  fresh = passes("fresh").measured()["channels"][0]["dropouts"]
  assert fresh["per_minute"] <= 2.0
  p = passes("worn")
  found = p.measured()["channels"][0]["dropouts"]["events"]
  # truth dropouts deeper than 6 db, in parts of the test file with content
  sos = signal.butter(2, (300.0, 3000.0), btype="bandpass", fs=RATE, output="sos")
  want = signal.sosfiltfilt(sos, p.test[:, 0])
  w = int(0.01 * RATE)
  n = len(want) // w
  level = 20 * np.log10(np.sqrt(np.mean(want[: n * w].reshape(n, w) ** 2, axis=1)) + 1e-12)
  content = level > testfile.REF_DBFS - 3.0103 + measure.DROPOUT_CONTENT_DB
  pos, off = p.truth["pos"], p.truth["offset"]
  hit = total = 0
  for at, length, depth in p.truth["dropouts"]:
    k = int(at - off + length / 2)
    if depth < 6 or not 0 <= k < len(pos):
      continue
    t = pos[k] / RATE
    if not content[min(int(t / 0.01), n - 1)]:
      continue
    total += 1
    hit += any(abs(e["time"] + e["seconds"] / 2 - t) < 0.06 for e in found)
  assert total >= 20
  assert hit / total >= 0.8


@pytest.mark.slow
def test_m8_print_through(passes):
  # on a deck where it can be heard: the ancient fake's echo is only ~3 db
  # over its hiss, with its pitch wandering out of the band (02-plan.md)
  p = passes("print-through")
  pt = p.measured()["channels"][0]["print_through"]
  bp = signal.butter(2, [200.0, 3000.0], btype="bandpass", fs=RATE, output="sos")
  gain = 20 * np.log10(abs(signal.sosfreqz(bp, worN=[2 * np.pi * 3150 / RATE])[1][0]))
  assert pt["found"]
  assert abs(pt["period_s"] - p.deck.print_through_s) <= 0.05
  assert abs(pt["post_db"] - (p.deck.print_through_db - 6.0 + gain)) <= 2.0
  assert abs(pt["pre_db"] - (p.deck.print_through_db + gain)) <= 2.0


@pytest.mark.slow
def test_m9_ancient_wow_from_the_tone(passes):
  p = passes("ancient")
  wf = p.measured()["channels"][0]["wow_flutter"]
  cut = measure._Cutter(p.alignment.recording[:, 0], p.alignment.tracks[0], RATE)
  wow, _, speed = _truth_speed_bands(p, cut)
  assert abs(wf["speed"] - speed) <= 2e-4
  assert abs(wf["wow_rms"] / wow - 1) <= 0.2
