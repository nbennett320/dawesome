import { BEATS_IN_VIEW, gridSteps, maxScale, openingScale, panForWidth, visibleBeats } from './grid'
import { MAX_PIXELS_PER_BEAT, MIN_LINE_GAP } from '../../../editing/beatGrid'

describe('gridSteps', () => {
  it('splits beats into finer lines when there is room', () => {
    // a 960px playlist showing 16 beats: 60px a beat, a line every 1/16 note
    expect(gridSteps(60, 4)).toEqual({ lineEvery: 1 / 4, labelEvery: 4 })
    // zoomed in further: still 1/16 notes, the finest there is
    expect(gridSteps(160, 4).lineEvery).toBe(1 / 4)
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
    // and on a narrow canvas: bar lines thin out too, to every 4 bars
    expect(gridSteps(300 / 256, 4)).toEqual({ lineEvery: 16, labelEvery: 32 })
  })

  it('always numbers a beat that has a line', () => {
    [0.5, 1, 3, 7, 9, 20, 27, 29, 60, 200].forEach(pixelsPerBeat => {
      [3, 4].forEach(beatsPerBar => {
        const { lineEvery, labelEvery } = gridSteps(pixelsPerBeat, beatsPerBar)
        expect(labelEvery % lineEvery).toBe(0)
        expect(labelEvery % beatsPerBar).toBe(0)
        expect(lineEvery * pixelsPerBeat).toBeGreaterThanOrEqual(MIN_LINE_GAP)
      })
    })
  })

  it('numbers bars, not beats, however far it zooms in', () => {
    [30, 60, 200, 2000].forEach(pixelsPerBeat => {
      expect(gridSteps(pixelsPerBeat, 4).labelEvery).toBe(4)
      expect(gridSteps(pixelsPerBeat, 3).labelEvery).toBe(3)
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

describe('maxScale', () => {
  it('stops where a beat is MAX_PIXELS_PER_BEAT wide', () => {
    const scale = maxScale(960, 256)
    expect(960 * scale / 256).toBeCloseTo(MAX_PIXELS_PER_BEAT)
    expect(gridSteps(960 * scale / 256, 4).lineEvery).toBe(1 / 4)
  })

  it('lets the playlist open where it does', () => {
    expect(maxScale(960, 256)).toBeGreaterThan(openingScale(256))
  })

  it('never stops short of the whole playlist', () => {
    // 4 beats on a 2000px canvas: past the limit even zoomed all the way out
    expect(maxScale(2000, 4)).toBe(1)
  })

  it('has no limit without a width to work from', () => {
    expect(maxScale(0, 256)).toBe(Infinity)
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

describe('panForWidth', () => {
  it('keeps the beat at the left edge when the canvas is resized', () => {
    // zoomed 4x: the left edge is a quarter of the way along a 800px canvas
    const scale = 4
    const leftFraction = (transformX: number, width: number) => -transformX / (width * scale)
    const pan = panForWidth(-800, 800, 1000)
    expect(leftFraction(pan, 1000)).toBeCloseTo(leftFraction(-800, 800))
  })

  it('leaves the pan alone without a width to scale from', () => {
    expect(panForWidth(-120, 0, 600)).toBe(-120)
    expect(panForWidth(-120, 600, 0)).toBe(-120)
  })

  it('leaves an unpanned playlist unpanned', () => {
    expect(panForWidth(0, 600, 900)).toBe(0)
  })
})
