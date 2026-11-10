import { playedRegion, timeAtX, xAtTime } from './region'

describe('playedRegion', () => {
  it('is the whole canvas with no window', () => {
    expect(playedRegion(null, 10, 500)).toEqual({ left: 0, width: 500 })
    expect(playedRegion(undefined, 10, 500)).toEqual({ left: 0, width: 500 })
  })

  it('places a window in the middle', () => {
    expect(playedRegion({ start: 2, end: 6 }, 10, 500)).toEqual({ left: 100, width: 200 })
  })

  it('runs to the end of the sample when end is null', () => {
    expect(playedRegion({ start: 4, end: null }, 10, 500)).toEqual({ left: 200, width: 300 })
  })

  it('starts at the left edge when start is 0', () => {
    expect(playedRegion({ start: 0, end: 5 }, 10, 500)).toEqual({ left: 0, width: 250 })
  })

  it('cuts a window reaching past the sample', () => {
    expect(playedRegion({ start: 8, end: 20 }, 10, 500)).toEqual({ left: 400, width: 100 })
    expect(playedRegion({ start: -3, end: 5 }, 10, 500)).toEqual({ left: 0, width: 250 })
  })

  it('never has a negative width', () => {
    expect(playedRegion({ start: 12, end: 15 }, 10, 500)).toEqual({ left: 500, width: 0 })
    expect(playedRegion({ start: 6, end: 2 }, 10, 500)).toEqual({ left: 300, width: 0 })
  })

  it('is the whole canvas for a zero-length sample', () => {
    expect(playedRegion({ start: 1, end: 2 }, 0, 500)).toEqual({ left: 0, width: 500 })
  })
})

describe('xAtTime and timeAtX', () => {
  const region = { left: 100, width: 200 }

  it('map the ends of the played part to the ends of the region', () => {
    expect(xAtTime(region, 4, 0)).toBe(100)
    expect(xAtTime(region, 4, 4)).toBe(300)
    expect(timeAtX(region, 4, 100)).toBe(0)
    expect(timeAtX(region, 4, 300)).toBe(4)
  })

  it('round trip inside the region', () => {
    [0, 0.5, 1.25, 3, 4].forEach(time => {
      expect(timeAtX(region, 4, xAtTime(region, 4, time))).toBeCloseTo(time)
    })
  })

  it('hold a pointer outside the region to its ends', () => {
    expect(timeAtX(region, 4, 0)).toBe(0)
    expect(timeAtX(region, 4, 450)).toBe(4)
  })

  it('cope with nothing played', () => {
    expect(xAtTime(region, 0, 1)).toBe(100)
    expect(timeAtX({ left: 100, width: 0 }, 4, 150)).toBe(0)
  })
})
