from dataclasses import replace

import numpy as np

from tape_model import faketape
from tape_model.dsp import RATE, from_db, read_at, rms, to_db
from tape_model.testfile import REF_DBFS, tone

# a deck that only moves time: no saturation, eq, noise or wobble
STILL = faketape.Deck(
  emphasis_db=0.0, headroom=1e6, asymmetry=0.0, compress=0.0, head_bump_db=0.0,
  hf_loss_hz=23_000.0, low_cut_hz=1.0, crosstalk_db=-200.0,
  speed_error=0.0, wow_depth=0.0, flutter=(), drift_depth=0.0, scrape_depth=0.0,
  hiss_dbfs=-300.0, out_gain_db=0.0,
)


def sine(freq, seconds, dbfs=REF_DBFS):
  x = tone(freq, seconds, dbfs)
  return np.stack([x, x], axis=1)


def peak_hz(x, rate=RATE):
  spectrum = np.abs(np.fft.rfft(x * np.hanning(len(x)), 8 * len(x)))
  return np.argmax(spectrum) * rate / (8 * len(x))


def test_read_at_whole_positions_is_the_signal():
  x = np.random.default_rng(0).standard_normal((1000, 2))
  assert np.allclose(read_at(x, np.arange(1000.0)), x)


def test_read_at_outside_is_silent():
  x = np.ones((10, 2))
  assert np.all(read_at(x, np.array([-5.0, 20.0])) == 0.0)


def test_a_still_deck_only_delays():
  x = sine(1000.0, 1.0)
  out, truth = faketape.run(x, STILL)
  k = truth["offset"]
  # the low cut and lowpass are barely there, so it's the input, delayed
  assert np.max(np.abs(out[k + 1000:k + len(x) - 1000] - x[1000:-1000])) < 2e-3


def test_the_same_seed_makes_the_same_pass():
  x = sine(1000.0, 2.0)
  a, _ = faketape.run(x)
  b, _ = faketape.run(x)
  assert np.array_equal(a, b)


def test_running_fast_raises_the_pitch():
  x = sine(3150.0, 4.0)
  deck = replace(STILL, speed_error=0.01)
  out, truth = faketape.run(x, deck)
  k = truth["offset"]
  assert abs(peak_hz(out[k + RATE:k + 3 * RATE, 0]) / 3150.0 - 1.01) < 5e-4
  # and the truth says it read through the input 1% faster
  slope = np.polyfit(np.arange(len(truth["pos"])), truth["pos"], 1)[0]
  assert abs(slope - 1.01) < 1e-6


def test_the_truth_lines_the_output_up_with_the_input():
  x = sine(1000.0, 3.0)
  deck = replace(STILL, wow_depth=2e-3, flutter=((7.0, 5e-4),))
  out, truth = faketape.run(x, deck)
  k = truth["offset"]
  # undo the wobble with the truth: the input comes back
  expected = read_at(x, truth["pos"])
  n = len(truth["pos"])
  assert np.max(np.abs(out[k + 1000:k + n - 1000] - expected[1000:-1000])) < 2e-3


def test_drive_saturates():
  x = sine(1000.0, 1.0)
  clean, t = faketape.run(x, replace(faketape.Deck(), hiss_dbfs=-300.0, drive_db=0.0))
  hot, _ = faketape.run(x, replace(faketape.Deck(), hiss_dbfs=-300.0, drive_db=18.0))
  k = t["offset"]
  gain = to_db(rms(hot[k + RATE // 4:k + RATE // 2])) - to_db(rms(clean[k + RATE // 4:k + RATE // 2]))
  # 18 db more in, noticeably less than 18 more out
  assert 3.0 < gain < 15.0


def test_worn_tape_has_dropouts():
  x = sine(1000.0, 30.0)
  deck = replace(faketape.worn(), hiss_dbfs=-300.0, dropouts_per_minute=60.0)
  out, truth = faketape.run(x, deck)
  k = truth["offset"]
  # a sine's level, 10 ms at a time: some windows dip well under the rest
  body = out[k + RATE:k + 29 * RATE, 0]
  levels = np.sqrt(np.mean(body[:len(body) // 480 * 480].reshape(-1, 480) ** 2, axis=1))
  assert np.min(levels) < np.median(levels) * from_db(-3.0)


def test_hiss_is_at_its_level():
  x = np.zeros((10 * RATE, 2))
  deck = replace(faketape.Deck(), out_gain_db=0.0)
  out, _ = faketape.run(x, deck)
  assert abs(to_db(rms(out[:, 0])) - deck.hiss_dbfs) < 0.5


def test_ancient_tape_is_mono():
  x = np.random.default_rng(1).standard_normal((RATE, 2)) * 0.05
  out, _ = faketape.run(x, faketape.preset("ancient"))
  assert np.array_equal(out[:, 0], out[:, 1])


def test_print_through_echoes_a_reel_turn_either_side():
  deck = replace(STILL, print_through_db=-34.0)
  x = np.zeros((6 * RATE, 2))
  burst = sine(1000.0, 0.2)
  at = 3 * RATE
  x[at:at + len(burst)] = burst
  out, truth = faketape.run(x, deck)
  k, d = truth["offset"], int(deck.print_through_s * RATE)

  def level(start):
    return to_db(rms(out[k + start + 1000:k + start + len(burst) - 1000, 0]))

  main = level(at)
  # the pre-echo a turn early, the post-echo 6 db quieter a turn late
  assert abs(level(at - d) - (main - 34.0)) < 1.5
  assert abs(level(at + d) - (main - 40.0)) < 1.5
  # and nothing between
  assert level(at - d // 2) < main - 100.0


def test_stick_slips_dip_the_speed():
  deck = replace(STILL, stick_slips_per_minute=60.0)
  v = faketape.speed(30 * RATE, deck, np.random.default_rng(0))
  assert v.max() <= 1.0 + 1e-12
  assert v.min() < 0.98


def test_modulation_noise_rides_the_signal():
  deck = replace(STILL, modulation_noise_db=-26.0)
  quiet = np.zeros((RATE, 2))
  loud = sine(1000.0, 1.0, -6.0)
  out_quiet, _ = faketape.run(quiet, deck)
  out_loud, t = faketape.run(loud, deck)
  assert np.max(np.abs(out_quiet)) < 1e-9
  k = t["offset"]
  residual = out_loud[k + 2000:k + RATE - 2000, 0] - loud[2000:RATE - 2000, 0]
  assert -40.0 < to_db(rms(residual)) - to_db(rms(loud[:, 0])) < -15.0
