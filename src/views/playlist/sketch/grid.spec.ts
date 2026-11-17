import {
  BEATS_IN_VIEW, fitSpan, MIN_LABEL_GAP, gridSteps, maxScale, openingScale, panForWidth, panToShow, rulerLabel, scrollToShow, visibleBeats,
} from './grid'
import { MAX_PIXELS_PER_BEAT, MIN_LINE_GAP } from '../../../editing/beatGrid'

describe('gridSteps', () => {
  it('splits beats into finer lines when there is room', () => {
    // a 960px playlist showing 16 beats: 60px a beat, a line every 1/16 note
    expect(gridSteps(60, 4)).toEqual({ lineEvery: 1 / 4, labelEvery: 1 })
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
    // 128 bars of 4/4 across 960px: 1.875px a beat, 7.5px a bar
    const steps = gridSteps(960 / 512, 4)
    expect(steps.lineEvery).toBe(8)
    expect(steps.labelEvery).toBe(16)
    // and on a narrow canvas: to every 8 bars
    expect(gridSteps(300 / 512, 4)).toEqual({ lineEvery: 32, labelEvery: 64 })
  })

  it('always numbers a beat that has a line', () => {
    [0.5, 1, 3, 7, 9, 20, 27, 29, 60, 200].forEach(pixelsPerBeat => {
      [3, 4].forEach(beatsPerBar => {
        const { lineEvery, labelEvery } = gridSteps(pixelsPerBeat, beatsPerBar)
        expect(labelEvery % lineEvery).toBe(0)
        // every beat, or whole bars
        expect(labelEvery === 1 || labelEvery % beatsPerBar === 0).toBe(true)
        expect(lineEvery * pixelsPerBeat).toBeGreaterThanOrEqual(MIN_LINE_GAP)
        // and the numbers have room
        expect(labelEvery * pixelsPerBeat).toBeGreaterThanOrEqual(MIN_LABEL_GAP)
      })
    })
  })

  it('numbers beats once the grid is down to 1/8 notes', () => {
    // 30px a beat: 1/8 note lines
    expect(gridSteps(30, 4)).toEqual({ lineEvery: 1 / 2, labelEvery: 1 })
    expect(gridSteps(30, 3).labelEvery).toBe(1)
    // 25px a beat: lines on beats, numbers on bars only
    expect(gridSteps(25, 4)).toEqual({ lineEvery: 1, labelEvery: 4 })
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

describe('rulerLabel', () => {
  it('names bars by number and beats as bar.beat, from 1', () => {
    expect(rulerLabel(0, 4)).toBe('1')
    expect(rulerLabel(1, 4)).toBe('1.2')
    expect(rulerLabel(3, 4)).toBe('1.4')
    expect(rulerLabel(4, 4)).toBe('2')
    expect(rulerLabel(9, 4)).toBe('3.2')
  })

  it('follows the bar length', () => {
    expect(rulerLabel(2, 3)).toBe('1.3')
    expect(rulerLabel(3, 3)).toBe('2')
  })
})

describe('visibleBeats', () => {
  it('covers the canvas, with a beat to spare', () => {
    // 60px a beat, 960px wide, not panned: beats 0 to 16
    expect(visibleBeats(60, 0, 960, 512)).toEqual([0, 17])
    // panned 8 bars along
    expect(visibleBeats(60, -32 * 60, 960, 512)).toEqual([31, 49])
  })

  it('keeps a number that still shows at the left edge', () => {
    // 7.5px a beat, panned to beat 6: the number at beat 4 (28px wide at
    // most) still reaches onto the canvas
    const [first] = visibleBeats(7.5, -6 * 7.5, 960, 512, 28)
    expect(first).toBeLessThanOrEqual(4)
    expect(visibleBeats(7.5, -6 * 7.5, 960, 512)[0]).toBe(5)
  })

  it('stays within the playlist', () => {
    expect(visibleBeats(60, -506 * 60, 960, 512)).toEqual([505, 512])
    expect(visibleBeats(0, 0, 960, 512)).toEqual([0, 0])
  })
})

describe('maxScale', () => {
  it('stops where a beat is MAX_PIXELS_PER_BEAT wide', () => {
    const scale = maxScale(960, 512)
    expect(960 * scale / 512).toBeCloseTo(MAX_PIXELS_PER_BEAT)
    expect(gridSteps(960 * scale / 512, 4).lineEvery).toBe(1 / 4)
  })

  it('lets the playlist open where it does', () => {
    expect(maxScale(960, 512)).toBeGreaterThan(openingScale(512))
  })

  it('never stops short of the whole playlist', () => {
    // 4 beats on a 2000px canvas: past the limit even zoomed all the way out
    expect(maxScale(2000, 4)).toBe(1)
  })

  it('has no limit without a width to work from', () => {
    expect(maxScale(0, 512)).toBe(Infinity)
  })
})

describe('openingScale', () => {
  it('shows the first four bars of a 128 bar playlist', () => {
    expect(openingScale(512)).toBe(512 / BEATS_IN_VIEW)
    expect(openingScale(512)).toBe(32)
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

describe('panToShow', () => {
  // a 1000px canvas zoomed in 4 times: the playlist is 4000px across
  const show = (pan: number, x: number) => panToShow(pan, x, 4, 1000, 80)

  it('keeps a pan that already shows it', () => {
    expect(show(-400, 200)).toBe(-400)
  })

  it('pans just far enough to bring it in from either side', () => {
    // at 2000 (screen 1600): 80px in from the right
    expect(show(-400, 500)).toBe(-1080)
    // at 400 (screen 0): 80px in from the left
    expect(show(-1600, 400)).toBe(-1520)
  })

  it("doesn't pan past the playlist's ends", () => {
    expect(show(-400, 0)).toBe(0)
    expect(show(0, 1000)).toBe(-3000)
  })

  it('keeps the margin within a quarter of a narrow canvas', () => {
    expect(panToShow(0, 100, 4, 200, 80)).toBe(-250)
  })
})

describe('scrollToShow', () => {
  // the canvas shows from 30 (under the ruler) to 330
  const show = (scroll: number, top: number) => scrollToShow(scroll, top, top + 100, 30, 330)

  it('keeps a scroll that already shows the lane', () => {
    expect(show(0, 50)).toBe(0)
    expect(show(100, 300)).toBe(100)
  })

  it('scrolls just far enough to show it below the ruler, or above the bottom', () => {
    expect(show(100, 110)).toBe(80)
    expect(show(0, 400)).toBe(170)
  })

  it('shows a lane taller than the view from its top', () => {
    expect(scrollToShow(0, 400, 900, 30, 330)).toBe(370)
  })
})

describe('fitSpan', () => {
  // a 1000px canvas, a 400 beat playlist (2.5px a beat unzoomed), 40px in
  const fit = (start: number, end: number, most = 100) => fitSpan(start, end, 400, 1000, 40, most)

  it('fits the span across the view, centred', () => {
    const { scale, pan } = fit(100, 200)
    // 100 beats into 920px
    expect(scale).toBeCloseTo(3.68)
    // its middle (beat 150) in the view's middle
    expect(pan + 150 / 400 * 1000 * scale).toBeCloseTo(500)
  })

  it('stops at the most it can zoom in, and at the whole playlist', () => {
    expect(fit(100, 101, 20).scale).toBe(20)
    expect(fit(0, 400).scale).toBe(1)
    expect(fit(0, 400).pan).toBe(0)
  })

  it("doesn't pan past the playlist's start", () => {
    expect(fit(0, 20).pan).toBe(0)
  })
})
