import { buildPeaks, peakRange, peaksOfPairs } from './peaks'

// the same answer the slow way
const bruteRange = (ys: number[], from: number, to: number): [number, number] | null => {
  const run = ys.slice(Math.max(0, Math.floor(from)), Math.min(ys.length - 1, Math.floor(to)) + 1)
  return run.length ? [Math.min(...run), Math.max(...run)] : null
}

// repeatable "random" levels
const levels = (n: number, seed = 1) => {
  let s = seed
  return Array.from({ length: n }, () => {
    s = (s * 16807) % 2147483647
    return Math.fround(s / 2147483647)
  })
}

describe('peakRange', () => {
  it('matches the lowest and highest point of every run', () => {
    // a power of two, an odd count, and a count one past a power of two
    [64, 37, 17].forEach(n => {
      const ys = levels(n, n)
      const peaks = buildPeaks(ys)

      for(let from = 0; from < n; from++) {
        for(let to = from; to < n; to++) {
          expect(peakRange(peaks, from, to)).toEqual(bruteRange(ys, from, to))
        }
      }
    })
  })

  it('keeps runs within the waveform', () => {
    const ys = levels(10)
    const peaks = buildPeaks(ys)

    expect(peakRange(peaks, -5, 3)).toEqual(bruteRange(ys, 0, 3))
    expect(peakRange(peaks, 8, 50)).toEqual(bruteRange(ys, 8, 9))
    expect(peakRange(peaks, 12, 20)).toBeNull()
    expect(peakRange(buildPeaks([]), 0, 4)).toBeNull()
  })

  it('works on a single point', () => {
    expect(peakRange(buildPeaks([0.25]), 0, 0)).toEqual([0.25, 0.25])
  })
})

describe('peaksOfPairs', () => {
  it('keeps the y of each [x, y] pair', () => {
    const peaks = peaksOfPairs([0, 0.5, 1, 0.25, 2, 0.75])
    expect(peaks.count).toBe(3)
    expect(Array.from(peaks.levels[0].min)).toEqual([0.5, 0.25, 0.75])
    expect(peakRange(peaks, 0, 2)).toEqual([0.25, 0.75])
  })
})
