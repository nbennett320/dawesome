# training the tape core on aligned passes, and the linear baseline it has
# to beat
#
# the loss is error-to-signal twice: once pre-emphasized (the highs, where
# saturation shows) and once plain (the lows: the head bump and low cut
# live under 200 hz, which pre-emphasis would all but ignore). each window
# runs `warm` frames first that don't count, so the state has settled (the
# fake deck's low cut and squash remember tens of milliseconds)

import time

import numpy as np
from scipy import signal

from . import model as tape_model
from .dataset import Windows

PRE_EMPHASIS = 0.85


# the spectral loss's sizes: a multi-resolution stft (short for timing,
# long for the harmonics' fine lines)
STFT_SIZES = (256, 1024, 4096)


def _stft_loss(torch, out, target):
  # spectral convergence plus log magnitude, at several sizes. on a log
  # scale a harmonic 45 db down counts as much as the fundamental's shape;
  # error-to-signal all but ignores it (G5's T6)
  total = 0.0
  for size in STFT_SIZES:
    window = torch.hann_window(size)
    spec = lambda x: torch.stft(x, size, size // 4, window=window, return_complex=True).abs()
    a, b = spec(out), spec(target)
    convergence = torch.linalg.norm(a - b) / (torch.linalg.norm(b) + 1e-10)
    log = torch.mean(torch.abs(torch.log(a + 1e-5) - torch.log(b + 1e-5)))
    total = total + convergence + log
  return total / len(STFT_SIZES)


def _loss(torch, out, target, stft_weight=0.0):
  def esr(a, b):
    return torch.sum((a - b) ** 2) / (torch.sum(b ** 2) + 1e-10)

  def pre(v):
    return v[:, 1:] - PRE_EMPHASIS * v[:, :-1]

  loss = esr(pre(out), pre(target)) + esr(out, target)
  if stft_weight > 0:
    loss = loss + stft_weight * _stft_loss(torch, out, target)
  return loss


def train(passes, rate, held_out_spans, minutes=30.0, hidden=32, seed=1,
          length=4096, warm=4096, batch=32, log=print, stft_weight=0.0):
  torch = tape_model._torch()
  torch.set_num_threads(max(1, torch.get_num_threads()))
  net = tape_model.build(hidden, seed)
  train_windows = Windows(passes, length, warm, held_out_spans, rate, seed)
  opt = torch.optim.Adam(net.parameters(), lr=5e-3)
  budget = minutes * 60.0
  start = time.time()
  history = []
  step = 0
  best = (np.inf, None)
  while True:
    elapsed = time.time() - start
    if elapsed > budget:
      break
    # the learning rate falls from 5e-3 to 1e-4 over the time budget
    frac = elapsed / budget
    for g in opt.param_groups:
      g["lr"] = 1e-4 + (5e-3 - 1e-4) * 0.5 * (1 + np.cos(np.pi * frac))
    x, d, y = (torch.from_numpy(a) for a in train_windows.batch(batch))
    out, _ = net(x, d)
    loss = _loss(torch, out[:, warm:], y[:, warm:], stft_weight)
    opt.zero_grad()
    loss.backward()
    torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
    opt.step()
    step += 1
    value = float(loss.detach())
    history.append((step, elapsed, value))
    # keep the best by a running mean of the training loss (validation is
    # the held-out material, judged afterwards, never chosen on)
    if step % 50 == 0:
      recent = float(np.mean([h[2] for h in history[-50:]]))
      if recent < best[0]:
        best = (recent, {k: v.clone() for k, v in net.state_dict().items()})
      log(f"step {step:5d}  {elapsed / 60:5.1f} min  loss {recent:.5f}")
  if best[1] is not None:
    net.load_state_dict(best[1])
  return net, {"steps": step, "minutes": (time.time() - start) / 60.0, "history": history[::10],
               "windows": len(train_windows), "hidden": hidden, "length": length, "warm": warm, "batch": batch,
               "stft_weight": stft_weight}


# --- the linear baseline --------------------------------------------------

class Linear:
  # the best purely linear filter from input to target: the cross-spectrum
  # over the input's spectrum, from the same usable, not-held-out stretches
  # the network trains on (with each pass's drive applied to the input)
  def __init__(self, passes, rate, held_out_spans, size=1 << 14):
    sxx = np.zeros(size // 2 + 1)
    sxy = np.zeros(size // 2 + 1, dtype=complex)
    window = np.hanning(size)
    for p in passes:
      g = 10 ** (p.drive_db / 20)
      for c in range(p.inputs.shape[1]):
        ok = p.usable[:, c].copy()
        for a, b in held_out_spans:
          ok[int(a * rate):int(b * rate)] = False
        run = np.concatenate([[0], np.cumsum(ok)])
        for s in range(0, len(ok) - size, size // 2):
          if run[s + size] - run[s] < size:
            continue
          X = np.fft.rfft(p.inputs[s:s + size, c] * g * window)
          Y = np.fft.rfft(p.targets[s:s + size, c] * window)
          sxx += np.abs(X) ** 2
          sxy += np.conj(X) * Y
    # where the input hardly reaches (between the content's 12 khz cut and
    # the pilot, say) the ratio is hiss over nothing: regularized
    response = sxy / (sxx + 1e-6 * sxx.max() + 1e-20)
    self.impulse = np.roll(np.fft.irfft(response, size), size // 2)

  def __call__(self, audio, drive_db):
    x = np.asarray(audio, dtype=np.float64) * 10 ** (drive_db / 20)
    return signal.fftconvolve(x, self.impulse, mode="full")[len(self.impulse) // 2:][:len(x)]
