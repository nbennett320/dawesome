import json

import numpy as np
import pytest

from tape_model import testfile
from tape_model.dsp import RATE, from_db, rms, to_db


@pytest.fixture(scope="module")
def built():
  return testfile.build(seed=1)


def band_db(x, low, high, rate=RATE):
  spectrum = np.abs(np.fft.rfft(x * np.hanning(len(x))))
  freqs = np.fft.rfftfreq(len(x), 1.0 / rate)
  return to_db(np.sqrt(np.sum(spectrum[(freqs >= low) & (freqs < high)] ** 2)))


def cut(audio, section, rate=RATE, margin=0.05):
  return audio[int((section.start + margin) * rate):int((section.end - margin) * rate)]


def test_the_same_seed_makes_the_same_file(built):
  again, _ = testfile.build(seed=1)
  assert np.array_equal(built[0], again)


def test_it_fits_without_clipping(built):
  audio, test_map = built
  assert audio.shape[1] == 2
  assert np.max(np.abs(audio)) < from_db(-1.0)
  assert 200 < test_map.seconds < 300


def test_sections_are_in_order_and_inside_the_file(built):
  audio, test_map = built
  ends = [0.0]
  for s in test_map.sections:
    assert s.start >= ends[-1] - 1e-9
    assert s.end > s.start
    ends.append(s.end)
  assert ends[-1] <= len(audio) / RATE + 1e-9


def test_the_calibration_tone_is_at_the_reference(built):
  audio, test_map = built
  x = cut(audio, test_map.section("calibration"))[:, 0]
  # a sine's rms is its peak less 3 db; the pilot adds next to nothing
  assert abs(to_db(rms(x)) - (testfile.REF_DBFS - 3.01)) < 0.05


def test_the_pilot_runs_under_the_music(built):
  audio, test_map = built
  x = cut(audio, test_map.section("music"))[:, 0]
  pilot = band_db(x, testfile.PILOT_HZ - 50, testfile.PILOT_HZ + 50)
  # the music is low-passed: next to nothing near the pilot but the pilot
  near = band_db(x, 13_500, testfile.PILOT_HZ - 200)
  assert pilot - near > 40


def test_the_pilot_is_off_in_the_preroll(built):
  audio, test_map = built
  assert np.max(np.abs(cut(audio, test_map.section("preroll")))) == 0.0


def test_sweep_gaps_cover_the_pilot_and_harmonics(built):
  _, test_map = built
  gaps = test_map.pilot["gaps"]
  assert len(gaps) == len(testfile.SWEEP_DBFS) * testfile.PILOT_GAP_HARMONICS
  for level in testfile.SWEEP_DBFS:
    sweep = test_map.section(f"sweep_{int(level)}")
    mine = [g for g in gaps if sweep.start <= g[0] < sweep.end]
    assert len(mine) == testfile.PILOT_GAP_HARMONICS
    for a, b in mine:
      assert sweep.start < a < b <= sweep.end
    # the fundamental's gap is where the sweep is between 13 and 17 khz
    k = testfile.SWEEP_SECONDS / np.log(1000.0)
    a, b = max(mine)
    assert abs(a - (sweep.start + k * np.log(13_000 / 20))) < 1e-9
    assert abs(b - (sweep.start + k * np.log(17_000 / 20))) < 1e-9


def test_left_and_right_only_are_one_sided(built):
  audio, test_map = built
  left = cut(audio, test_map.section("left_only"))
  # the other side only has the pilot
  assert band_db(left[:, 1], 900, 1100) < band_db(left[:, 0], 900, 1100) - 80


def test_the_map_survives_json(built):
  _, test_map = built
  back = testfile.Map.from_json(json.loads(json.dumps(test_map.to_json())))
  assert back == test_map
