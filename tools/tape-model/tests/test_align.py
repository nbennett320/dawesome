# alignment, against the fake deck's truth. the criteria (c1-c8) are in
# context/tape-project/02-plan.md, written before the code ran

import numpy as np
import pytest

from tape_model import align, faketape
from tape_model.dsp import RATE, rms, sinc_read, to_db
from tests.conftest import built


def errors(p, channel):
  # each grid point's position against the truth, less the constant
  # (the tape's own delay) taken where the pilot is tracked
  t = p.alignment.tracks[channel]
  k = t.grid - p.truth["offset"]
  ok = (k >= 0) & (k < len(p.truth["pos"]))
  true = np.full(len(t.grid), np.nan)
  true[ok] = p.truth["pos"][k[ok]] - (p.deck.channel_delay if channel == 1 else 0.0)
  e = t.pos - true
  best = 2 if (t.quality == 2).any() else 1
  const = np.nanmedian(e[t.quality >= best])
  return e - const, t.quality, ok, const


# --- quick: the pieces -----------------------------------------------------

def test_a_bridge_snaps_to_the_right_cycle():
  # a smooth wobble, cut by a gap, its far side off by 3 cycles and a bit
  period = RATE / 15_000.0
  grid = np.arange(0, 4 * RATE, align.GRID)
  true = grid * 1.001 + 8.0 * np.sin(2 * np.pi * 0.7 * grid / RATE)
  a, b = 3000, 3060
  prev = (0, a, true[:a])
  nxt = (b, len(grid), true[b:] - 3 * period + 0.03)
  k, miss, curve, raw, gap = align._bridge(grid, prev, nxt, period, RATE)
  assert abs(k - 3 * period) < 1e-9
  assert miss < 0.05
  assert np.max(np.abs(curve - true[a:b])) < 0.05


def test_local_cleaning_drops_a_wrong_cluster():
  centers = np.arange(0, 2 * RATE, 480)
  residual = np.sin(np.arange(len(centers)) / 20.0)
  residual[100:103] += 40.0
  ok = align._clean_local(centers, residual, np.ones(len(centers), dtype=bool), RATE)
  assert not ok[100:103].any()
  assert ok[:90].all()


def test_polarity_by_content():
  test, test_map = built()
  sync = align.Sync(2 * RATE, test_map.section("sync_end").start * RATE, 1.0, [False],
                    test_map.section("sync_start").start * RATE, test_map.section("sync_end").start * RATE)
  band = align._band(test.mean(axis=1), RATE)
  sync.start, sync.end = sync.test_start, sync.test_end
  assert align.polarity(test[:, 0], band, sync, RATE) == 1.0
  assert align.polarity(-test[:, 0], band, sync, RATE) == -1.0


def test_two_passes_in_one_file_is_an_error():
  test, test_map = built()
  twice = np.concatenate([test, np.zeros((RATE, 2)), test])
  with pytest.raises(align.AlignError, match="one pass per file"):
    align.find_sync(twice, test, test_map)


def test_no_chirps_is_an_error():
  test, test_map = built()
  with pytest.raises(align.AlignError, match="sync chirps"):
    align.find_sync(np.zeros((10 * RATE, 2)), test, test_map)


# --- slow: whole passes ----------------------------------------------------

@pytest.mark.slow
def test_c1_speed_from_the_chirps(passes):
  p = passes("fresh")
  s, off, pos = p.alignment.sync, p.truth["offset"], p.truth["pos"]
  k0, k1 = int(s.start - off), int(s.end - off)
  assert abs(s.speed - (pos[k1] - pos[k0]) / (k1 - k0)) < 2e-5


@pytest.mark.slow
@pytest.mark.parametrize("channel", [0, 1])
def test_c2_tracked_positions(passes, channel):
  e, q, ok, _ = errors(passes("fresh"), channel)
  sel = ok & (q == 2)
  assert sel.mean() > 0.9
  assert np.sqrt(np.mean(e[sel] ** 2)) <= 0.05
  assert np.max(np.abs(e[sel])) <= 0.5


@pytest.mark.slow
@pytest.mark.parametrize("channel", [0, 1])
def test_c2b_bridged_positions(passes, channel):
  e, q, ok, _ = errors(passes("fresh"), channel)
  assert np.max(np.abs(e[ok & (q == 1)])) <= 1.0


@pytest.mark.slow
def test_c3_bridges_inside_sweeps_snap(passes):
  # every gap inside a sweep is bridged by the sweep's own track. (gaps
  # past a sweep's end now start a new island, placed by an anchor burst)
  for t in passes("fresh").alignment.tracks:
    sweeps = [b for b in t.bridges if b["how"] == "sweep"]
    assert sweeps and not any(b["flagged"] for b in sweeps)


@pytest.mark.slow
@pytest.mark.parametrize("channel", [0, 1])
def test_c4_aligned_music_against_the_core(passes, channel):
  p = passes("fresh")
  _, _, _, const = errors(p, channel)
  music = p.map.section("music")
  a, b = int((music.start + 0.5) * RATE), int((music.end - 0.5) * RATE)
  core = faketape.core(p.test, p.deck)[:, channel] * 10 ** (p.deck.out_gain_db / 20)
  want = sinc_read(core, np.arange(a, b) - const)
  got = p.alignment.aligned[a:b, channel]
  assert to_db(rms(got - want)) - to_db(rms(want)) <= -40.0


@pytest.mark.slow
def test_c5_the_delay_between_channels(passes):
  delays = passes("fresh").alignment.summary()["channels"]
  assert abs(delays[1]["delay_frames"] - 0.7) <= 0.05


@pytest.mark.slow
def test_c6_worn_tracked_positions(passes):
  p = passes("worn")
  e, q, ok, _ = errors(p, 0)
  sel = ok & (q == 2)
  assert np.sqrt(np.mean(e[sel] ** 2)) <= 0.05 and np.max(np.abs(e[sel])) <= 0.5
  music = p.map.section("music")
  t = p.alignment.tracks[0]
  inside = (t.pos / RATE > music.start) & (t.pos / RATE < music.end)
  assert np.mean(t.quality[inside] == 2) >= 0.9


@pytest.mark.slow
@pytest.mark.parametrize("channel", [0, 1])
def test_c7_worn_music_with_dropouts_masked(passes, channel):
  p = passes("worn")
  _, _, _, const = errors(p, channel)
  music = p.map.section("music")
  a, b = int((music.start + 0.5) * RATE), int((music.end - 0.5) * RATE)
  core = faketape.core(p.test, p.deck)[:, channel] * 10 ** (p.deck.out_gain_db / 20)
  want = sinc_read(core, np.arange(a, b) - const)
  got = p.alignment.aligned[a:b, channel]
  mask = np.ones(b - a, dtype=bool)
  pos, off = p.truth["pos"], p.truth["offset"]
  for at, length, _ in p.truth["dropouts"]:
    k0, k1 = int(at - off), int(at - off + length)
    if k1 < 0 or k0 >= len(pos):
      continue
    m0, m1 = int(pos[max(k0, 0)]), int(pos[min(k1, len(pos) - 1)])
    mask[max(0, m0 - a - 960):max(0, m1 - a + 960)] = False
  assert to_db(rms((got - want)[mask])) - to_db(rms(want[mask])) <= -33.0


@pytest.mark.slow
def test_c8_ancient_content_tracking_only(passes):
  # sanity bounds only: no pilot gets through this tape, and its wobble and
  # wear defeat content tracking for most of the pass (see 02-plan.md)
  p = passes("ancient")
  s = p.alignment.summary()
  assert all(not ch["pilot"] for ch in s["channels"])
  assert s["inverted"] == [False, False]
  off, pos = p.truth["offset"], p.truth["pos"]
  k0, k1 = int(p.alignment.sync.start - off), int(p.alignment.sync.end - off)
  assert abs(p.alignment.sync.speed - (pos[k1] - pos[k0]) / (k1 - k0)) < 1e-4
  # what it does call aligned is near: content-tracked positions through
  # this much filtering carry a few frames of group delay that depends on
  # what's playing (5.3 measured; the bound is a sanity bound, see 02-plan)
  e, q, ok, _ = errors(p, 0)
  assert np.nanmax(np.abs(e[ok & (q >= 1)])) <= 8.0


@pytest.mark.slow
def test_quality_stays_honest_at_full_drive(passes):
  # +18 db crushes the pilot under loud content: less is aligned, but what
  # is, is labelled truthfully
  for channel in (0, 1):
    e, q, ok, _ = errors(passes("fresh+18"), channel)
    assert np.max(np.abs(e[ok & (q == 2)])) <= 0.5
    assert np.max(np.abs(e[ok & (q == 1)])) <= 5.0


@pytest.mark.slow
def test_an_inverted_channel_is_turned_back(passes):
  p = passes("fresh")
  test, test_map = built()
  flipped = p.rec * np.array([1.0, -1.0])
  result = align.align(flipped, RATE, test, test_map)
  assert result.inverted == [False, True]
  # the same alignment as the right-way-up pass, within a hundredth of a frame
  for a, b in zip(result.tracks, p.alignment.tracks):
    both = (a.quality == 2) & (b.quality == 2)
    assert both.mean() > 0.9
    assert np.max(np.abs(a.pos[both] - b.pos[both])) < 0.01


@pytest.mark.slow
def test_a_pilot_on_its_own_track_survives_full_drive(passes):
  # T15: with the pilot on track 4, loud content can't crush it. on the
  # mixed layout only 2 % of the music aligned at +18 db
  p = passes("4track+18")
  assert p.alignment.channels == [0, 1]
  music = p.map.section("music")
  for i, track in enumerate(p.alignment.tracks):
    k = track.grid - p.truth["offset"]
    ok = (k >= 0) & (k < len(p.truth["pos"]))
    e = np.full(len(track.grid), np.nan)
    e[ok] = track.pos[ok] - (p.truth["pos"][k[ok]] - i * p.deck.channel_delay)
    e -= np.nanmedian(e[track.quality == 2])
    inside = ok & (track.pos / RATE > music.start) & (track.pos / RATE < music.end)
    assert np.mean(track.quality[inside] == 2) >= 0.95
    assert np.nanmax(np.abs(e[ok & (track.quality == 2)])) <= 0.5
  delays = p.alignment.summary()["channels"]
  assert abs(delays[1]["delay_frames"] - 0.7) <= 0.05
