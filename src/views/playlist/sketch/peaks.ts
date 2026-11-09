// a waveform's levels, summarised so the lowest and highest level across any
// run of points can be found in a few steps, however many points it spans.
// used to draw a column of pixels at a time rather than a line per point
//
// levels[0] holds the points themselves. each level after it halves the
// one before: bucket b of level k covers points b * 2^k up to (b + 1) * 2^k
export interface Peaks {
  // how many points
  count: number
  levels: Array<{ min: Float32Array, max: Float32Array }>
}

// summarise a waveform's levels (its y values, in order)
export const buildPeaks = (ys: ArrayLike<number>): Peaks => {
  const first = Float32Array.from(ys)
  const levels = [{ min: first, max: first }]

  for(let below = levels[0]; below.min.length > 1; below = levels[levels.length - 1]) {
    const size = Math.ceil(below.min.length / 2)
    const min = new Float32Array(size)
    const max = new Float32Array(size)

    for(let b = 0; b < size; b++) {
      // an odd point out at the end has no partner
      const pair = Math.min(2 * b + 1, below.min.length - 1)
      min[b] = Math.min(below.min[2 * b], below.min[pair])
      max[b] = Math.max(below.max[2 * b], below.max[pair])
    }

    levels.push({ min, max })
  }

  return { count: first.length, levels }
}

// the y values of a waveform's [x, y] pairs (see calc_waveform_from_samples)
export const peaksOfPairs = (pairs: ArrayLike<number>): Peaks => {
  const ys = new Float32Array(Math.floor(pairs.length / 2))
  for(let i = 0; i < ys.length; i++) ys[i] = pairs[2 * i + 1]
  return buildPeaks(ys)
}

// the lowest and highest level among points `from` to `to`, both included
// (kept within the waveform). null when that leaves no points
export const peakRange = (peaks: Peaks, from: number, to: number): [number, number] | null => {
  let i = Math.max(0, Math.floor(from))
  const end = Math.min(peaks.count - 1, Math.floor(to)) + 1
  if(i >= end) return null

  let low = Infinity
  let high = -Infinity

  // take the biggest bucket that starts at i and fits before the end
  while(i < end) {
    // level k's buckets are `size` (2^k) points wide
    let k = 0
    let size = 1
    while(
      k + 1 < peaks.levels.length
      && i % (size * 2) === 0
      && i + size * 2 <= end
    ) {
      k += 1
      size *= 2
    }

    // i is a multiple of size, so this is a whole bucket
    const b = i / size
    low = Math.min(low, peaks.levels[k].min[b])
    high = Math.max(high, peaks.levels[k].max[b])
    i += size
  }

  return [low, high]
}
