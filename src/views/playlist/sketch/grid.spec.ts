import { BEATS_IN_VIEW, gridSteps, openingScale, visibleBeats } from './grid'

describe('gridSteps', () => {
  it('draws every beat when there is room', () => {
    // a 960px playlist showing 16 beats: 60px a beat
    expect(gridSteps(60, 4)).toEqual({ lineEvery: 1, labelEvery: 1 })
  })

  it('thins the numbers before the lines', () => {
    // 20px a beat: lines still fit, numbers only on bars
    expect(gridSteps(20, 4)).toEqual({ lineEvery: 1, labelEvery: 4 })
  })

  it('drops beat lines once they would smear together', () => {
    // 4px a beat: a bar is 16px
    expect(gridSteps(4, 4)).toEqual({ lineEvery: 4, labelEvery: 8 })
  })

  it('thins bars when the whole playlist is on screen', () => {
    // 256 beats across 960px: 3.75px a beat, 15px a bar
    const steps = gridSteps(960 / 256, 4)
    expect(steps.lineEvery).toBe(4)
    expect(steps.labelEvery).toBe(8)
    // and on a narrow canvas: every bar line stays, the numbers thin out
    expect(gridSteps(300 / 256, 4)).toEqual({ lineEvery: 4, labelEvery: 32 })
  })

  it('always numbers a beat that has a line', () => {
    [0.5, 1, 3, 7, 9, 20, 27, 29, 60].forEach(pixelsPerBeat => {
      [3, 4].forEach(beatsPerBar => {
        const { lineEvery, labelEvery } = gridSteps(pixelsPerBeat, beatsPerBar)
        expect(labelEvery % lineEvery).toBe(0)
        expect(lineEvery).toBeLessThanOrEqual(beatsPerBar)
      })
    })
  })

  it('follows the bar length', () => {
    // three beats to the bar, 5px a beat: a line every bar, a number every other
    expect(gridSteps(5, 3)).toEqual({ lineEvery: 3, labelEvery: 6 })
  })

  it('never returns a step of zero', () => {
    expect(gridSteps(0, 4).lineEvery).toBeGreaterThan(0)
    expect(gridSteps(NaN, 4).labelEvery).toBeGreaterThan(0)
  })
})

describe('visibleBeats', () => {
  it('covers the canvas, with a beat to spare', () => {
    // 60px a beat, 960px wide, not panned: beats 0 to 16
    expect(visibleBeats(60, 0, 960, 256)).toEqual([0, 17])
    // panned 8 bars along
    expect(visibleBeats(60, -32 * 60, 960, 256)).toEqual([31, 49])
  })

  it('keeps a number that still shows at the left edge', () => {
    // 7.5px a beat, panned to beat 6: the number at beat 4 (28px wide at
    // most) still reaches onto the canvas
    const [first] = visibleBeats(7.5, -6 * 7.5, 960, 256, 28)
    expect(first).toBeLessThanOrEqual(4)
    expect(visibleBeats(7.5, -6 * 7.5, 960, 256)[0]).toBe(5)
  })

  it('stays within the playlist', () => {
    expect(visibleBeats(60, -250 * 60, 960, 256)).toEqual([249, 256])
    expect(visibleBeats(0, 0, 960, 256)).toEqual([0, 0])
  })
})

describe('openingScale', () => {
  it('shows the first four bars of a 64 bar playlist', () => {
    expect(openingScale(256)).toBe(256 / BEATS_IN_VIEW)
    expect(openingScale(256)).toBe(16)
  })

  it('never zooms out past the whole playlist', () => {
    expect(openingScale(8)).toBe(1)
  })
})
