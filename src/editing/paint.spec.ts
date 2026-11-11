import { paintBars, paintSpans } from './paint'

describe('paintSpans', () => {
  it('lays the pattern end to end over the drag', () => {
    expect(paintSpans(4, 12, 4, 0.25)).toEqual([{ start: 4, end: 8 }, { start: 8, end: 12 }])
  })

  it('cuts the last one short where the drag ends', () => {
    expect(paintSpans(0, 10, 4, 0.25)).toEqual([
      { start: 0, end: 4 }, { start: 4, end: 8 }, { start: 8, end: 10 },
    ])
    expect(paintSpans(2, 3, 4, 0.25)).toEqual([{ start: 2, end: 3 }])
  })

  it('is the same dragged backwards', () => {
    expect(paintSpans(10, 0, 4, 0.25)).toEqual(paintSpans(0, 10, 4, 0.25))
  })

  it('paints one whole clip for a click', () => {
    expect(paintSpans(6, 6, 4, 0.25)).toEqual([{ start: 6, end: 10 }])
    expect(paintSpans(6, 6.1, 4, 0.25)).toEqual([{ start: 6, end: 10 }])
  })

  it('leaves off an end piece too short to be a clip', () => {
    expect(paintSpans(0, 8.1, 4, 0.25)).toEqual([{ start: 0, end: 4 }, { start: 4, end: 8 }])
  })

  it('never starts before the playlist does', () => {
    expect(paintSpans(-2, 4, 4, 0.25)).toEqual([{ start: 0, end: 4 }])
  })

  it('paints nothing of a pattern with no length', () => {
    expect(paintSpans(0, 8, 0, 0.25)).toEqual([])
  })
})

describe('paintBars', () => {
  it('is enough bars to hold the drag', () => {
    expect(paintBars(0, 4, 4, 64)).toBe(1)
    expect(paintBars(0, 4.5, 4, 64)).toBe(2)
    expect(paintBars(8, 0, 4, 64)).toBe(2)
    expect(paintBars(0, 9, 3, 64)).toBe(3)
  })

  it('is at least one, and at most as many as a pattern can be', () => {
    expect(paintBars(3, 3, 4, 64)).toBe(1)
    expect(paintBars(0, 1000, 4, 64)).toBe(64)
  })
})
