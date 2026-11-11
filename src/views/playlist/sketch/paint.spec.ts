import { paintedSpans } from './paint'

describe('paintedSpans', () => {
  it('lays a pattern end to end, the last cut short', () => {
    // a one bar pattern of 4/4: sixteen steps, four beats
    expect(paintedSpans(4, 14, 16, 4)).toEqual([
      { start: 4, end: 8 }, { start: 8, end: 12 }, { start: 12, end: 14 },
    ])
  })

  it('paints one whole clip of the pattern for a click', () => {
    expect(paintedSpans(6, 6, 32, 4)).toEqual([{ start: 6, end: 14 }])
  })

  it('with no pattern, paints one clip from start to end', () => {
    expect(paintedSpans(2, 8, null, 4)).toEqual([{ start: 2, end: 8 }])
    expect(paintedSpans(8, 2, null, 4)).toEqual([{ start: 2, end: 8 }])
  })

  it('with no pattern, a click paints a bar', () => {
    expect(paintedSpans(20, 20, null, 4)).toEqual([{ start: 20, end: 24 }])
    expect(paintedSpans(20, 20, null, 3)).toEqual([{ start: 20, end: 23 }])
  })
})
