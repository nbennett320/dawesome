# lining a recording of the test file up with the original
#
# a pass through tape comes back late, at a slightly wrong speed, wobbling,
# maybe upside down. this finds where every frame of the recording came
# from in the test file, and resamples the recording back onto the test
# file's timeline, so the network learns the tape and nothing else:
#
# 1. the sync chirps: where the pass starts and ends, its mean speed, and
#    whether it's inverted
# 2. content tracking: short stretches of the recording cross-correlated
#    with the test file, coarse to fine. works on most content but not on
#    pure tones or silence, and only to a fraction of a sample
# 3. the pilot tone's phase: the wobble in fine detail wherever the pilot
#    comes through. across a gap (a sweep passing it, a dropout) a curve
#    fitted on both sides bridges it, snapped to a whole pilot cycle
#
# each channel is lined up on its own: on a multitrack deck the tracks reach
# the heads a hair apart (azimuth), and that difference belongs to the deck,
# not to the training data
#
# a position is in test-file frames, a recording frame is `n`. GRID frames
# apart, every channel keeps where it came from and how sure that is:
# 2 the pilot's phase, 1 a bridge or content tracking, 0 a guess

from dataclasses import dataclass, field
from fractions import Fraction

import numpy as np
from scipy import signal

from .dsp import sinc_read

GRID = 24
# content tracking's band: where tape changes the sound least
TRACK_BAND = (300.0, 3000.0)
# (window, hop, search, strict), frames, coarse to fine. the coarse passes
# only find roughly where things are, so they take broad peaks: through
# heavy wobble (1 % on the ancient fake) a long window smears the peak over
# tens of frames. only the last pass's points are kept, and it's strict
TRACK_PASSES = ((8192, 2400, 512, False), (4096, 960, 128, False), (2048, 480, 64, True))
# before them, the loudness envelopes (40 hz smooth, at 1 khz) in long
# windows: wobble smears a recording's fine detail, but drum hits and the
# edges of bursts still line up. (window, hop, search) in envelope frames
ENVELOPE_RATE = 1000
ENVELOPE_PASS = (2000, 500, 150)
ENVELOPE_MIN_PEAK = 0.5
TRACK_MIN_PEAK = 0.4
TRACK_MIN_PEAK_COARSE = 0.25
# a second peak this close to the best makes a window ambiguous (a tone)
TRACK_AMBIGUOUS = 0.8
# a window of the test file that's a tone (or a few) can't be placed by
# content, whatever the tape did: it is, when this share of its energy is in
# this share of its spectrum
TRACK_TONAL_SHARE = 0.8
TRACK_TONAL_BINS = 0.03
# and a peak this broad is no use: this many frames either side still this
# high (tape's phase, like the ancient fake's, broadens peaks a lot)
TRACK_SHARP_LAG = 4
TRACK_SHARP = 0.95
TRACK_MIN_DBFS = -60.0
PILOT_BANDWIDTH = 400.0
# positions from the pilot are smoothed this much: wobble faster than this
# moves the tape by under a hundredth of a frame (1e-4 of speed at 200 hz
# is 0.004 frames), but interference near the pilot (loud drums through
# tape) jitters its phase. measuring flutter uses the pilot unsmoothed
PILOT_SMOOTH_HZ = 60.0
# where the pilot's noise is measured, this far above it
PILOT_NOISE_OFFSET = 700.0
PILOT_MIN_SNR_DB = 20.0
PILOT_SEGMENT_MIN_S = 0.1
# loud passages squash the pilot and wiggle its frequency without hurting
# its phase (checked against the fake deck's truth: under 0.04 cycles
# through the music). its phase can only slip a cycle when something as
# loud as it cancels it, and then its level collapses: so it's only cut
# under this much of its usual level, or of its own running level. a
# frequency excursion against its running median only marks it less sure
PILOT_MIN_LEVEL = 0.25
PILOT_MIN_LOCAL = 0.25
PILOT_UNSURE_HZ = 40.0
PILOT_LOCAL_S = 0.1
# how far either side of a gap in the map, and of anything unreliable
PILOT_GAP_MARGIN_S = 0.05
PILOT_PAD_S = 0.005
PILOT_EDGE_S = 0.005
# a bridge's context either side is twice the gap, within these, and its
# curve's degree grows with the gap: short gaps need a short, simple fit
# (flutter isn't smooth over half a second)
BRIDGE_CONTEXT_S = (0.015, 0.5)
BRIDGE_MIN_CONTEXT_S = 0.01
BRIDGE_DEGREES = ((0.1, 2), (0.4, 3), (np.inf, 5))
# a curve can't be trusted to a whole pilot cycle across more than this:
# over half a second, wow and drift move further than it can see (the fake
# deck's truth caught a 9-cycle snap across 0.55 s). a sweep's own track
# bridges any gap inside the sweep; past this, a new island starts, placed
# by its own content
BRIDGE_MAX_GAP_S = 0.15
# a filled stretch counts (quality 1) only between two points this close
# that agree this well
FILL_CLOSE_S = 0.03
FILL_AGREE_FRAMES = 2.0
BRIDGE_MAX_ESTIMATE_GAP_S = 6.0
# a bridge further than this from a whole pilot cycle is flagged
BRIDGE_MAX_MISS = 0.25
# content points an island needs before its constant counts
ISLAND_MIN_POINTS = 20


# a sync chirp's correlation envelope must reach this
SYNC_MIN_PEAK = 0.15
# a second pair of chirps this close in level to the loudest is a second pass
SYNC_SECOND_PASS_DB = 6.0


class AlignError(Exception):
  pass


@dataclass
class Sync:
  # where the template's start chirp begins in the recording (frames)
  start: float
  end: float
  # test-file frames per recording frame, on average
  speed: float
  # each channel's chirp sign: only a fallback, see polarity()
  inverted: list
  # the test file's chirps, for the linear map
  test_start: float
  test_end: float

  def pos(self, n):
    return self.test_start + (np.asarray(n, dtype=np.float64) - self.start) * self.speed

  def frame(self, pos):
    return self.start + (np.asarray(pos, dtype=np.float64) - self.test_start) / self.speed


@dataclass
class Track:
  # every GRID recording frames: where each came from, and how sure
  grid: np.ndarray
  pos: np.ndarray
  quality: np.ndarray
  pilot: bool
  pilot_snr_db: float
  bridges: list = field(default_factory=list)
  # content tracking's points: recording frame, position, usable
  content: tuple = ()
  # the pilot mixed down, at the grid (for the delay between channels)
  pilot_z: np.ndarray = None


@dataclass
class Alignment:
  sync: Sync
  tracks: list
  rate: int
  # the recording on the test file's timeline, and how sure each GRID
  # frames of it is, per channel
  aligned: np.ndarray
  quality: np.ndarray
  # a pilot cycle, in frames
  period: float
  # which channels came back upside down
  inverted: list
  # the recording at the test file's rate, each channel the right way up
  recording: np.ndarray = None

  def summary(self):
    delays = channel_delays(self.tracks, self.period)
    return {
      "rate": self.rate,
      "speed": self.sync.speed,
      "speed_error": self.sync.speed - 1.0,
      "inverted": self.inverted,
      "start_frame": self.sync.frame(0.0).item(),
      "channels": [
        {
          "pilot": t.pilot,
          "pilot_snr_db": t.pilot_snr_db,
          "pilot_tracked": float(np.mean(t.quality == 2)),
          "bridges": len(t.bridges),
          "flagged_bridges": sum(1 for b in t.bridges if b["flagged"]),
          "delay_frames": d,
        }
        for t, d in zip(self.tracks, delays)
      ],
    }


def _mono(x):
  return x.mean(axis=1) if x.ndim == 2 else x


def _parabolic(y, i):
  if i <= 0 or i >= len(y) - 1:
    return float(i)
  a, b, c = y[i - 1], y[i], y[i + 1]
  den = a - 2 * b + c
  return float(i) + (0.5 * (a - c) / den if den != 0 else 0.0)


# the recording at the test file's rate
def match_rate(x, rate, to_rate):
  if rate == to_rate:
    return x
  ratio = Fraction(to_rate, rate).limit_denominator(1000)
  return signal.resample_poly(x, ratio.numerator, ratio.denominator, axis=0)


# --- 1. the sync chirps ---------------------------------------------------

def _normalized_correlation(x, template):
  corr = signal.correlate(x, template, mode="valid", method="fft")
  energy = np.concatenate([[0.0], np.cumsum(x * x)])
  local = energy[len(template):] - energy[:-len(template)]
  return corr / np.sqrt(np.maximum(local, 1e-20) * np.sum(template * template))


def find_sync(rec, test, test_map):
  rate = test_map.rate
  first, last = test_map.section("sync_start"), test_map.section("sync_end")
  a, b = int(round(first.start * rate)), int(round(first.end * rate))
  template = _mono(test)[a:b]
  test_start, test_end = first.start * rate, last.start * rate
  spacing = test_end - test_start

  # by the correlation's envelope: tape turns a chirp's phase (the ancient
  # fake deck's flips its sign), which moves and shrinks the signed peak.
  # each channel on its own, envelopes added: a track wired backwards
  # would cancel the chirps in a plain sum of the channels
  chans = rec if rec.ndim == 2 else rec[:, None]
  ncorrs = [_normalized_correlation(chans[:, c], template) for c in range(chans.shape[1])]
  envelope = np.mean([np.abs(signal.hilbert(n)) for n in ncorrs], axis=0)
  peaks, props = signal.find_peaks(envelope, height=SYNC_MIN_PEAK, distance=rate)
  order = np.argsort(props["peak_heights"])[::-1][:20]
  peaks = peaks[order]
  pairs = [
    (p, q) for p in peaks for q in peaks
    if q > p and 0.97 <= spacing / (q - p) <= 1.03
  ]
  if not pairs:
    raise AlignError("couldn't find both sync chirps the right distance apart: is this one whole pass of the test file?")
  # print-through leaves quiet ghosts of each chirp a reel's turn either
  # side, which correlate as well as the real ones: the loudest pair is
  # the pass, and only a second pair about as loud means a second pass
  energy = np.concatenate([[0.0], np.cumsum(np.sum(chans * chans, axis=1))])

  def loudness(at):
    return 10 * np.log10((energy[at + len(template)] - energy[at]) / len(template) + 1e-30)

  pairs.sort(key=lambda pq: -min(loudness(pq[0]), loudness(pq[1])))
  if len(pairs) > 1 and min(loudness(pairs[1][0]), loudness(pairs[1][1])) > min(loudness(pairs[0][0]), loudness(pairs[0][1])) - SYNC_SECOND_PASS_DB:
    raise AlignError(f"found {len(pairs)} pairs of sync chirps about as loud: record one pass per file")
  p, q = pairs[0]
  # each channel's chirp sign, only a fallback: polarity() votes properly
  inverted = [bool(n[p] < 0) for n in ncorrs]

  # again with the chirp stretched to the pass's speed, to a fraction of
  # a frame (the envelope's peak: the linear map needs no better)
  speed = spacing / (q - p)
  stretched = sinc_read(template, np.arange(int(len(template) / speed)) * speed)

  def refine(at):
    lo = max(0, at - 256)
    local = np.mean([
      np.abs(signal.hilbert(_normalized_correlation(chans[lo:at + 256 + len(stretched), c], stretched)))
      for c in range(chans.shape[1])
    ], axis=0)
    return lo + _parabolic(local, int(np.argmax(local)))

  start, end = refine(p), refine(q)
  return Sync(start, end, spacing / (end - start), inverted, test_start, test_end)


# which way up a recording is, +1 or -1 (0 if it can't tell): long windows
# of it, cross-correlated with the test file around where the linear map
# puts them, each voting with its strongest peak's sign
def polarity(x, test_band, sync, rate, windows=24, win=16384, search=4096):
  rec_band = _band(x, rate)
  centers = np.linspace(sync.start + win, sync.end - win, windows).astype(np.int64)
  votes = 0.0
  nfft = 1 << int(np.ceil(np.log2(win + 2 * search)))
  for c in centers:
    p0 = int(round(float(sync.pos(c))))
    r = rec_band[c - win // 2:c + win // 2]
    lo = p0 - win // 2 - search
    if lo < 0 or lo + win + 2 * search > len(test_band) or len(r) < win:
      continue
    t = test_band[lo:lo + win + 2 * search]
    corr = np.fft.irfft(np.conj(np.fft.rfft(r, nfft)) * np.fft.rfft(t, nfft), nfft)[:2 * search + 1]
    norm = np.sqrt(np.sum(r * r) * np.sum(t * t) * win / len(t)) + 1e-30
    best = int(np.argmax(np.abs(corr)))
    if abs(corr[best]) / norm > 0.3:
      votes += np.sign(corr[best]) * abs(corr[best]) / norm
  return float(np.sign(votes))


# --- 2. content tracking --------------------------------------------------

# the tracking band, whitened a little (a first difference), so content
# with energy across it correlates to a sharp peak
def _band(x, rate):
  sos = signal.butter(4, TRACK_BAND, btype="bandpass", fs=rate, output="sos")
  return np.diff(signal.sosfiltfilt(sos, x), prepend=0.0)


def _xcorr_windows(rec, test, centers, predicted, win, search, min_rms, strict=True):
  half = win // 2
  span = win + 2 * search
  nfft = 1 << int(np.ceil(np.log2(span)))
  rec_pad = np.concatenate([np.zeros(span), rec, np.zeros(span)])
  test_pad = np.concatenate([np.zeros(span), test, np.zeros(span)])
  out_pos = np.zeros(len(centers))
  out_ok = np.zeros(len(centers), dtype=bool)
  batch = max(1, (1 << 22) // nfft)
  lags = np.arange(2 * search + 1)
  for s in range(0, len(centers), batch):
    c = centers[s:s + batch]
    p0 = np.rint(predicted[s:s + batch]).astype(np.int64)
    r_idx = np.clip(c[:, None] - half + np.arange(win)[None, :] + span, 0, len(rec_pad) - 1)
    t_idx = np.clip(p0[:, None] - half - search + np.arange(span)[None, :] + span, 0, len(test_pad) - 1)
    r = rec_pad[r_idx]
    t = test_pad[t_idx]
    t_spec = np.fft.rfft(t, nfft)
    corr = np.fft.irfft(np.conj(np.fft.rfft(r, nfft)) * t_spec, nfft)[:, :2 * search + 1]
    # how tonal the test file is here
    power = np.sort(np.abs(t_spec) ** 2, axis=1)[:, ::-1]
    top = max(1, int(TRACK_TONAL_BINS * power.shape[1]))
    tonal = power[:, :top].sum(axis=1) > TRACK_TONAL_SHARE * (power.sum(axis=1) + 1e-30)
    energy = np.concatenate([np.zeros((len(c), 1)), np.cumsum(t * t, axis=1)], axis=1)
    t_energy = energy[:, lags + win] - energy[:, lags]
    r_energy = np.sum(r * r, axis=1)
    ncorr = corr / np.sqrt(np.maximum(t_energy * r_energy[:, None], 1e-30))
    best = np.argmax(ncorr, axis=1)
    rows = np.arange(len(c))
    peak = ncorr[rows, best]
    masked = ncorr.copy()
    mask = 16 if strict else max(16, win // 128)
    masked[np.abs(lags[None, :] - best[:, None]) <= mask] = -np.inf
    second = masked.max(axis=1)
    loud = np.sqrt(t_energy[rows, best] / win) > min_rms
    side = np.maximum(
      ncorr[rows, np.clip(best - TRACK_SHARP_LAG, 0, 2 * search)],
      ncorr[rows, np.clip(best + TRACK_SHARP_LAG, 0, 2 * search)],
    )
    sharp = side < TRACK_SHARP * peak
    inside = (best > TRACK_SHARP_LAG) & (best < 2 * search - TRACK_SHARP_LAG)
    if strict:
      ok = (peak >= TRACK_MIN_PEAK) & (second < TRACK_AMBIGUOUS * peak) & sharp & loud & inside & ~tonal
    else:
      ok = (peak >= TRACK_MIN_PEAK_COARSE) & (second < TRACK_AMBIGUOUS * peak) & loud & inside & ~tonal
    for i in range(len(c)):
      out_pos[s + i] = p0[i] - search + _parabolic(ncorr[i], int(best[i]))
    out_ok[s:s + batch] = ok
  return out_pos, out_ok


# the final pass's points must agree with their neighbours this close in
# time: wrong points come in clusters when tape crushes the content (+18 db
# drive on the fake deck), which a median over the nearest points misses
NEIGHBOUR_S = 0.1
NEIGHBOUR_MIN = 3
NEIGHBOUR_MAX_FRAMES = 1.5


def _clean_local(centers, residual, ok, rate):
  idx = np.flatnonzero(ok)
  keep = np.zeros(len(ok), dtype=bool)
  if len(idx) < NEIGHBOUR_MIN + 1:
    return keep
  n, r = centers[idx].astype(np.float64), residual[idx]
  reach = NEIGHBOUR_S * rate
  lo = np.searchsorted(n, n - reach)
  hi = np.searchsorted(n, n + reach, side="right")
  for j in range(len(idx)):
    near = np.concatenate([r[lo[j]:j], r[j + 1:hi[j]]])
    if len(near) < NEIGHBOUR_MIN:
      continue
    med = np.median(near)
    spread = 1.4826 * np.median(np.abs(near - med))
    if abs(r[j] - med) <= max(NEIGHBOUR_MAX_FRAMES, 3.0 * spread):
      keep[idx[j]] = True
  return keep


def _clean(centers, residual, ok):
  # drop points far from their neighbours' running median
  idx = np.flatnonzero(ok)
  if len(idx) < 5:
    return ok
  r = residual[idx]
  med = signal.medfilt(r, 7)
  mad = np.median(np.abs(r - med)) + 1e-9
  keep = np.abs(r - med) <= max(3.0 * 1.4826 * mad, 2.0)
  ok = ok.copy()
  ok[idx[~keep]] = False
  return ok


def _envelope(band, rate):
  step = rate // ENVELOPE_RATE
  sos = signal.butter(2, 40.0, fs=rate, output="sos")
  env = signal.sosfiltfilt(sos, np.abs(band))[::step]
  return env


# the envelope pass: (recording frames, positions) where it found a match
def _envelope_track(rec_band, test_band, sync, rate):
  step = rate // ENVELOPE_RATE
  er, et = _envelope(rec_band, rate), _envelope(test_band, rate)
  win, hop, search = ENVELOPE_PASS
  points = []
  for c in range(win // 2, len(er) - win // 2, hop):
    p0 = int(round(float(sync.pos(c * step)) / step))
    lo = p0 - win // 2 - search
    if lo < 0 or lo + win + 2 * search > len(et):
      continue
    r = er[c - win // 2:c + win // 2]
    t = et[lo:lo + win + 2 * search]
    r = r - r.mean()
    windows = np.lib.stride_tricks.sliding_window_view(t, win)
    windows = windows - windows.mean(axis=1, keepdims=True)
    corr = windows @ r / (np.sqrt(np.sum(windows * windows, axis=1) * np.sum(r * r)) + 1e-30)
    best = int(np.argmax(corr))
    # the next-highest separate peak (a smooth envelope's own peak is wide)
    tops, _ = signal.find_peaks(corr)
    others = corr[tops[np.abs(tops - best) > 10]]
    second = others.max() if len(others) else -1.0
    if corr[best] >= ENVELOPE_MIN_PEAK and second < TRACK_AMBIGUOUS * corr[best] and 0 < best < 2 * search:
      points.append((c * step, (lo + win // 2 + _parabolic(corr, best)) * step))
  return np.array(points).reshape(-1, 2)


# content tracking for one channel: points (recording frame, position, ok)
def track_content(rec, test_band, sync, rate):
  rec_band = _band(rec, rate)
  min_rms = 10.0 ** (TRACK_MIN_DBFS / 20.0)
  last = len(rec)

  def predictor(n):
    return sync.pos(n)

  coarse = _envelope_track(rec_band, test_band, sync, rate)
  if len(coarse) >= 2:
    ok = _clean(coarse[:, 0], coarse[:, 1] - sync.pos(coarse[:, 0]), np.ones(len(coarse), dtype=bool))
    if ok.sum() >= 2:
      kn, kr = coarse[ok, 0], coarse[ok, 1] - sync.pos(coarse[ok, 0])

      def predictor(n, kn=kn, kr=kr):
        return sync.pos(n) + np.interp(n, kn, kr)

  for win, hop, search, strict in TRACK_PASSES:
    centers = np.arange(win // 2, last - win // 2, hop, dtype=np.int64)
    predicted = predictor(centers)
    pos, ok = _xcorr_windows(rec_band, test_band, centers, predicted, win, search, min_rms, strict)
    ok = _clean(centers, pos - sync.pos(centers), ok)
    if strict:
      ok = _clean_local(centers, pos - sync.pos(centers), ok, rate)
    if ok.sum() >= 2:
      known_n, known_r = centers[ok].astype(np.float64), (pos - sync.pos(centers))[ok]

      def predictor(n, kn=known_n, kr=known_r):
        return sync.pos(n) + np.interp(n, kn, kr)
  return centers, pos, ok


# --- 3. the pilot ---------------------------------------------------------

def tone_phase(x, hz, rate, grid, margin=4800, chunk=1 << 21):
  # x mixed down by hz and low-passed with no phase shift, at the grid
  sos = signal.butter(4, PILOT_BANDWIDTH, fs=rate, output="sos")
  out = np.zeros(len(grid), dtype=np.complex128)
  w = 2 * np.pi * hz / rate
  for a in range(0, len(x), chunk):
    b = min(len(x), a + chunk)
    lo, hi = max(0, a - margin), min(len(x), b + margin)
    n = np.arange(lo, hi)
    z = signal.sosfiltfilt(sos, x[lo:hi] * np.exp(-1j * w * n))
    sel = (grid >= a) & (grid < b)
    out[sel] = z[grid[sel] - lo]
  return out


def _running_median(x, width):
  # a median over `width` points, quickly: of 8x-decimated blocks
  step = max(1, width // 8)
  blocks = x[: len(x) // step * step].reshape(-1, step).mean(axis=1) if step > 1 else x
  med = signal.medfilt(blocks, max(3, (width // step) | 1))
  return np.interp(np.arange(len(x)), np.arange(len(med)) * step + (step - 1) / 2, med)


def _smooth(pos, grid_rate):
  # low-passed with no phase shift, around its straight-line trend
  sos = signal.butter(2, PILOT_SMOOTH_HZ, fs=grid_rate, output="sos")
  if len(pos) <= 3 * (2 * len(sos) + 1) * 3:
    return pos
  t = np.arange(len(pos))
  line = np.polyval(np.polyfit(t, pos, 1), t)
  return line + signal.sosfiltfilt(sos, pos - line)


def _runs(mask):
  edges = np.diff(np.concatenate([[0], mask.astype(np.int8), [0]]))
  return list(zip(np.flatnonzero(edges == 1), np.flatnonzero(edges == -1)))


# the pilot's segments for one channel: [(grid start, grid end, relative
# position)], and its snr. a relative position is right up to a constant
def track_pilot(rec, sync, test_map, grid):
  rate = test_map.rate
  hz_test = test_map.pilot["hz"]
  hz = hz_test * sync.speed
  z = tone_phase(rec, hz, rate, grid)
  noise = np.abs(tone_phase(rec, hz + PILOT_NOISE_OFFSET, rate, grid))

  t = grid / rate
  span = (t >= sync.frame(test_map.pilot["start"] * rate) / rate + 0.05) & (t <= sync.frame(test_map.pilot["end"] * rate) / rate - 0.05)
  if not span.any():
    return [], -np.inf, None, None
  amp = np.abs(z)
  level = np.median(amp[span])
  floor = np.median(noise[span]) + 1e-30
  snr_db = 20 * np.log10(level / floor + 1e-30)
  if snr_db < PILOT_MIN_SNR_DB:
    return [], snr_db, None, None

  reliable = span & (amp > max(PILOT_MIN_LEVEL * level, 4.0 * floor))
  for g0, g1 in test_map.pilot["gaps"]:
    a = sync.frame(g0 * rate) / rate - PILOT_GAP_MARGIN_S
    b = sync.frame(g1 * rate) / rate + PILOT_GAP_MARGIN_S
    reliable &= ~((t >= a) & (t <= b))
  # against its own running frequency: wobble moves it slowly, something
  # beating with the pilot moves it fast
  local = int(PILOT_LOCAL_S * rate / GRID) | 1
  angle = np.unwrap(np.angle(z))
  freq = np.diff(angle, prepend=angle[0]) * (rate / GRID) / (2 * np.pi)
  excursion = np.abs(freq - _running_median(freq, local))
  reliable &= amp > PILOT_MIN_LOCAL * np.exp(_running_median(np.log(amp + 1e-30), local))
  # and a little either side of anything unreliable
  pad = int(PILOT_PAD_S * rate / GRID)
  widen = np.ones(2 * pad + 1)
  reliable &= np.convolve((~reliable).astype(float), widen, mode="same") == 0
  unsure = np.convolve((excursion >= PILOT_UNSURE_HZ).astype(float), widen, mode="same") > 0

  edge = int(PILOT_EDGE_S * rate / GRID)
  segments = []
  for a, b in _runs(reliable):
    a, b = a + edge, b - edge
    if (b - a) * GRID / rate < PILOT_SEGMENT_MIN_S:
      continue
    phase = np.unwrap(np.angle(z[a:b])) + 2 * np.pi * hz * grid[a:b] / rate
    segments.append((a, b, _smooth(phase * rate / (2 * np.pi * hz_test), rate / GRID)))
  return segments, snr_db, unsure, z


# a log sweep's phase at test-file frame p, and its speed (radians a frame)
def _sweep_phase(p, section, rate):
  f1, f2 = section.info["hz"]
  k = section.info["seconds"] / np.log(f2 / f1)
  t = (np.asarray(p) - section.start * rate) / rate
  return 2 * np.pi * f1 * k * (np.exp(t / k) - 1.0), 2 * np.pi * f1 * np.exp(t / k) / rate


# where a sweep says the recording came from, through the sweep: the
# recording mixed down by the sweep's expected phase. that phase moves
# slowly however far the guess is off, so it unwraps without slipping. it's
# right up to a constant phase c (where unwrapping started, and the tape's
# own phase), which as a position is c / the sweep's speed: it shrinks as
# the sweep rises. _sweep_bridge fits c from what the pilot already knows
SWEEP_TRACK_FROM_HZ = 500.0


def _sweep_track(rec, grid, sync, section, rate):
  f1, f2 = section.info["hz"]
  k = section.info["seconds"] / np.log(f2 / f1)
  begin = section.start + k * np.log(SWEEP_TRACK_FROM_HZ / f1)
  lo = np.searchsorted(grid, sync.frame(begin * rate))
  hi = np.searchsorted(grid, sync.frame((section.end - 0.1) * rate))
  if hi - lo < 10:
    return None
  n0, n1 = int(grid[lo]), int(grid[hi - 1]) + 1
  phase, _ = _sweep_phase(sync.pos(np.arange(n0, n1)), section, rate)
  sos = signal.butter(4, 100.0, fs=rate, output="sos")
  z = signal.sosfiltfilt(sos, rec[n0:n1] * np.exp(-1j * phase))[grid[lo:hi] - n0]
  _, speed = _sweep_phase(sync.pos(grid[lo:hi]), section, rate)
  amp = np.abs(z)
  ok = amp > 0.25 * np.median(amp)
  return lo, hi, sync.pos(grid[lo:hi]) + np.unwrap(np.angle(z)) / speed, ok, 1.0 / speed


def _bridge(grid, prev, nxt, period, rate):
  # one smooth curve through the end of `prev` and the start of `nxt`. both
  # are (grid start, grid end, position); nxt's is right up to a constant k.
  # returns k snapped to a whole pilot cycle, how far the fit missed that
  # (in cycles), and the curve across the gap. None if it can't be tried
  pa, pb, ppos = prev
  na, nb, npos = nxt
  gap_s = (grid[na] - grid[pb - 1]) / rate
  context = int(np.clip(gap_s, *BRIDGE_CONTEXT_S) * rate / GRID)
  ia = slice(max(0, (pb - pa) - context), pb - pa)
  ib = slice(0, min(nb - na, context))
  ga, gb = grid[pa:pb][ia], grid[na:nb][ib]
  if min(len(ga), len(gb)) * GRID / rate < BRIDGE_MIN_CONTEXT_S or gap_s > BRIDGE_MAX_ESTIMATE_GAP_S:
    return None
  degree = next(d for limit, d in BRIDGE_DEGREES if gap_s < limit)
  # a curve can't reach further than its context supports
  if min(len(ga), len(gb)) * GRID / rate < gap_s:
    degree = min(degree, 2)
  mid = 0.5 * (grid[pb - 1] + grid[na])
  scale = 0.5 * (gb[-1] - ga[0])
  powers = np.arange(degree + 1)
  ta, tb = (ga - mid) / scale, (gb - mid) / scale
  rows = np.vstack([
    np.hstack([ta[:, None] ** powers, np.zeros((len(ta), 1))]),
    np.hstack([tb[:, None] ** powers, -np.ones((len(tb), 1))]),
  ])
  # positions are big numbers: fit them relative to where prev ends
  base = ppos[ia][-1]
  values = np.concatenate([ppos[ia] - base, npos[ib] - base])
  coef, *_ = np.linalg.lstsq(rows, values, rcond=None)
  k = coef[-1]
  snapped = period * np.round(k / period)
  tg = (grid[pb:na] - mid) / scale
  curve = (tg[:, None] ** powers) @ coef[:-1] + base
  return snapped, abs(k - snapped) / period, curve, k, gap_s


# a bridge from a sweep's own track, if the gap is inside one: fit the
# track's constant phase and offset to everything the island already knows
# inside the sweep, then read the far side off it. the same answer as
# _bridge, or None
def _sweep_bridge(grid, island, nxt, period, rate, tracks):
  pb = island[-1][1]
  na, nb, npos = nxt
  context = int(0.05 * rate / GRID)
  for lo, hi, track, ok, inverse in tracks:
    if not (lo <= pb - context and na + context <= hi):
      continue
    idx, known = [], []
    for a, b, rel in island:
      i = np.arange(max(a, lo), min(b, hi))
      i = i[ok[i - lo]]
      idx.append(i)
      known.append(rel[i - a])
    idx, known = np.concatenate(idx), np.concatenate(known)
    right = np.arange(na, min(nb, na + context))
    right = right[ok[right - lo]]
    if len(idx) < context or len(right) < context // 2:
      return None
    # known = track + c * inverse + shift, over at least a little spread
    # of frequencies; otherwise just the shift
    base = known[-1]
    if (inverse[idx - lo].max() / inverse[idx - lo].min()) > 1.05:
      rows = np.stack([inverse[idx - lo], np.ones(len(idx))], axis=1)
      (c, shift), *_ = np.linalg.lstsq(rows, known - track[idx - lo] - base, rcond=None)
    else:
      c, shift = 0.0, float(np.median(known - track[idx - lo] - base))
    predicted = track + c * inverse + shift + base
    k = float(np.median(predicted[right - lo] - npos[right - na]))
    snapped = period * np.round(k / period)
    gap_s = (grid[na] - grid[pb - 1]) / rate
    return snapped, abs(k - snapped) / period, predicted[pb - lo:na - lo], k, gap_s
  return None


# everything for one channel
def track_channel(rec, test_band, sync, test_map):
  rate = test_map.rate
  grid = np.arange(0, len(rec), GRID, dtype=np.int64)
  linear = sync.pos(grid)
  centers, cpos, cok = track_content(rec, test_band, sync, rate)
  content_n = centers[cok].astype(np.float64)
  content_pos = cpos[cok]

  segments, snr_db, unsure, pilot_z = track_pilot(rec, sync, test_map, grid)
  sweeps = [
    t for t in (
      _sweep_track(rec, grid, sync, sec, rate)
      for sec in test_map.sections if sec.name.startswith("sweep")
    ) if t is not None
  ]
  period = rate / test_map.pilot["hz"]
  pos = np.full(len(grid), np.nan)
  quality = np.zeros(len(grid), dtype=np.int8)
  bridges = []

  if segments:
    # chain segments into islands: within one, every position shares one
    # unknown constant. a gap that can't be bridged to a whole cycle starts
    # a new island, linked to the last one only roughly (`soft`)
    islands = [[segments[0]]]
    soft = [None]
    for a, b, rel in segments[1:]:
      prev = islands[-1][-1]
      result = _sweep_bridge(grid, islands[-1], (a, b, rel), period, rate, sweeps)
      how = "sweep"
      # a sweep's track is as good as the pilot across any length of gap
      longest = np.inf
      if result is None:
        result = _bridge(grid, prev, (a, b, rel), period, rate)
        how = "curve"
        longest = BRIDGE_MAX_GAP_S
      bridge = {"start": int(grid[prev[1] - 1]), "end": int(grid[a]), "miss": None, "flagged": True, "how": how}
      if result is not None:
        k, miss, curve, raw, gap_s = result
        bridge["miss"] = float(miss)
        if miss <= BRIDGE_MAX_MISS and gap_s <= longest:
          bridge.update(flagged=False, island=len(islands) - 1, gap=(prev[1], a), curve=curve)
          islands[-1].append((a, b, rel + k))
      if bridge["flagged"]:
        islands.append([(a, b, rel)])
        soft.append(raw if result is not None else None)
      bridges.append(bridge)

    # each island's constant from content tracking. the best-covered one
    # sets the cycle lattice; the others snap to it if they're close
    def island_offset(island):
      diffs = []
      for a, b, rel in island:
        sel = (content_n >= grid[a]) & (content_n <= grid[b - 1])
        if sel.any():
          diffs.append(content_pos[sel] - np.interp(content_n[sel], grid[a:b], rel))
      d = np.concatenate(diffs) if diffs else np.array([])
      return (float(np.median(d)), len(d)) if len(d) else (None, 0)

    found = [island_offset(i) for i in islands]
    main = int(np.argmax([n for _, n in found]))
    if found[main][1] < ISLAND_MIN_POINTS:
      raise AlignError("the pilot is there, but too little content lines up with it")
    main_offset = found[main][0]
    offsets, sure = [None] * len(islands), [False] * len(islands)
    placed = [False] * len(islands)
    for i, (offset, count) in enumerate(found):
      if count < ISLAND_MIN_POINTS:
        continue
      snapped = main_offset + period * np.round((offset - main_offset) / period)
      sure[i] = abs(offset - snapped) <= BRIDGE_MAX_MISS * period
      offsets[i] = snapped if sure[i] else offset
      placed[i] = True
    # the rest from their rough links, forwards then backwards
    for _ in range(len(islands)):
      for i in range(1, len(islands)):
        if offsets[i] is None and offsets[i - 1] is not None and soft[i] is not None:
          offsets[i] = offsets[i - 1] + soft[i]
      for i in range(len(islands) - 2, -1, -1):
        if offsets[i] is None and offsets[i + 1] is not None and soft[i + 1] is not None:
          offsets[i] = offsets[i + 1] - soft[i + 1]
    # placed only roughly (by links): right in shape, but maybe not in
    # place, so not for training
    for island, offset, ok, put in zip(islands, offsets, sure, placed):
      if offset is None:
        continue
      for a, b, rel in island:
        pos[a:b] = rel + offset
        # an island whose content doesn't land on a whole pilot cycle has
        # something wrong in it: right in shape, not to be trusted in place
        quality[a:b] = np.where(unsure[a:b], 1, 2) if (ok and put) else 0
    # bridged gaps follow their fitted curve
    for bridge in bridges:
      if bridge["flagged"]:
        continue
      a, b = bridge.pop("gap")
      island = bridge.pop("island")
      curve = bridge.pop("curve")
      if offsets[island] is not None:
        pos[a:b] = curve + offsets[island]
        quality[a:b] = 1 if (placed[island] and sure[island]) else 0

  # everywhere else: through the content-tracking points inside the gap
  # and the known positions either side of it (content points are
  # positions too, just less exact ones: never bent to fit the edges).
  # quality 1 only between two of those close together that agree
  residual = pos - linear
  content_res = content_pos - sync.pos(content_n) if content_n.size else np.array([])
  known = ~np.isnan(residual)
  close = int(FILL_CLOSE_S * rate)
  for a, b in _runs(~known):
    inside = (content_n > grid[a] - GRID) & (content_n < grid[b - 1] + GRID) if content_n.size else np.array([], bool)
    xs, ys = list(content_n[inside]), list(content_res[inside])
    if a > 0:
      xs.insert(0, float(grid[a - 1]))
      ys.insert(0, residual[a - 1])
    if b < len(grid):
      xs.append(float(grid[b]))
      ys.append(residual[b])
    if not xs:
      residual[a:b] = 0.0
      continue
    xs, ys = np.array(xs), np.array(ys)
    residual[a:b] = np.interp(grid[a:b], xs, ys)
    j = np.clip(np.searchsorted(xs, grid[a:b]), 1, len(xs) - 1)
    near = (xs[j] - xs[j - 1] <= close) & (np.abs(ys[j] - ys[j - 1]) <= FILL_AGREE_FRAMES)
    quality[a:b] = np.where(near & (grid[a:b] >= xs[0]) & (grid[a:b] <= xs[-1]), 1, 0)
  pos = linear + residual

  # the tape only goes forwards
  pos = np.maximum.accumulate(pos)
  return Track(grid, pos, quality, bool(segments), float(snr_db), bridges, (centers, cpos, cok), pilot_z)


# how far each channel runs behind the first, in frames. content gives it
# roughly (each side's content weights the tape's phase differently, by a
# few hundredths of a frame); the pilot, the same on every channel, gives
# it exactly but only up to a whole pilot cycle, which content picks
def channel_delays(tracks, period):
  base = tracks[0]
  out = []
  for t in tracks:
    both = (base.quality == 2) & (t.quality == 2)
    if both.sum() < 100:
      both = (base.quality >= 1) & (t.quality >= 1)
    if not both.any():
      out.append(None)
      continue
    rough = float(np.median(base.pos[both] - t.pos[both]))
    if base.pilot_z is not None and t.pilot_z is not None:
      turn = np.angle(base.pilot_z[both] * np.conj(t.pilot_z[both]))
      fine = float(np.angle(np.mean(np.exp(1j * turn)))) / (2 * np.pi) * period
      rough = fine + period * np.round((rough - fine) / period)
    out.append(rough)
  return out


# the recording, channel by channel, on the test file's timeline
def dewobble(rec, tracks, sync, frames):
  rate_pad = 10 * 48_000
  aligned = np.zeros((frames, len(tracks)))
  quality = np.zeros(((frames + GRID - 1) // GRID, len(tracks)), dtype=np.int8)
  m = np.arange(frames, dtype=np.float64)
  for c, t in enumerate(tracks):
    grid = np.concatenate([[t.grid[0] - rate_pad], t.grid, [t.grid[-1] + rate_pad]]).astype(np.float64)
    pos = np.concatenate([[t.pos[0] - rate_pad * sync.speed], t.pos, [t.pos[-1] + rate_pad * sync.speed]])
    n = np.interp(m, pos, grid)
    aligned[:, c] = sinc_read(rec[:, c], n)
    idx = np.clip(np.rint(n[::GRID] / GRID).astype(np.int64), 0, len(t.quality) - 1)
    inside = (n[::GRID] >= 0) & (n[::GRID] < len(rec))
    quality[:, c] = np.where(inside, t.quality[idx], 0)
  return aligned, quality


def align(rec, rec_rate, test, test_map):
  rate = test_map.rate
  rec = match_rate(np.asarray(rec, dtype=np.float64), rec_rate, rate)
  if rec.ndim == 1:
    rec = rec[:, None]
  sync = find_sync(rec, test, test_map)
  test_band = _band(_mono(test), rate)
  # each channel the right way up on its own: a track can be wired backwards
  signs = [polarity(rec[:, c], test_band, sync, rate) or (-1.0 if sync.inverted[c] else 1.0) for c in range(rec.shape[1])]
  rec = rec * np.array(signs)[None, :]
  tracks = [track_channel(rec[:, c], test_band, sync, test_map) for c in range(rec.shape[1])]
  aligned, quality = dewobble(rec, tracks, sync, len(test))
  return Alignment(sync, tracks, rate, aligned, quality, rate / test_map.pilot["hz"], [s < 0 for s in signs], rec)
