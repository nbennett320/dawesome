import {
  GRID_GAP, MIN_LINE_GAP, adaptiveStep, gridStepOptions, lineLevel, stepLabel, tileStep,
} from './beatGrid'

describe('gridStepOptions', () => {
  it('goes from a 1/256 note to 512 bars, through half a bar in 4/4', () => {
    const steps = gridStepOptions(4)
    expect(steps[0]).toBe(1 / 64)
    expect(steps).toContain(1)
    expect(steps).toContain(2)
    expect(steps[steps.length - 1]).toBe(512 * 4)
  })

  it('goes from a beat straight to a bar in 3/4', () => {
    const steps = gridStepOptions(3)
    expect(steps[steps.indexOf(1) + 1]).toBe(3)
    expect(steps).toContain(6)
  })

  it('is finest first', () => {
    const steps = gridStepOptions(4)
    expect([...steps].sort((a, b) => a - b)).toEqual(steps)
  })
})

describe('adaptiveStep', () => {
  it('takes the step nearest GRID_GAP pixels', () => {
    // a beat 20px wide: a line every beat
    expect(adaptiveStep(GRID_GAP, 4)).toBe(1)
    // 80px a beat: a line every 1/16 note
    expect(adaptiveStep(80, 4)).toBe(1 / 4)
    // 5px a beat: a line every bar
    expect(adaptiveStep(5, 4)).toBe(4)
  })

  it('halves the step each time the zoom doubles', () => {
    for(let ppb = 1; ppb < 1000; ppb *= 2) {
      const step = adaptiveStep(ppb, 4)
      const finer = adaptiveStep(ppb * 2, 4)
      if(step > 1 / 64) expect(finer).toBe(step / 2)
    }
  })

  it('keeps lines between GRID_GAP/√2 and GRID_GAP×√2 apart', () => {
    // (within the steps there are: a 1/256 note to 512 bars)
    for(let ppb = 0.05; ppb < 1000; ppb *= 1.07) {
      const gap = adaptiveStep(ppb, 4) * ppb
      expect(gap).toBeGreaterThanOrEqual(GRID_GAP / Math.SQRT2 - 1e-9)
      expect(gap).toBeLessThanOrEqual(GRID_GAP * Math.SQRT2 + 1e-9)
    }
  })

  it('never draws lines closer than MIN_LINE_GAP', () => {
    [0.001, 0.3, 1, 7, 50, 100000].forEach(ppb => {
      [3, 4, 7].forEach(bar => {
        [-3, 0, 2].forEach(width => {
          const step = adaptiveStep(ppb, bar, width)
          // the coarsest step is the one exception: there's nothing coarser
          if(step !== 512 * bar) expect(step * ppb).toBeGreaterThanOrEqual(MIN_LINE_GAP)
        })
      })
    })
  })

  it('moves by a doubling for each step of width', () => {
    expect(adaptiveStep(GRID_GAP, 4, 1)).toBe(2)
    expect(adaptiveStep(GRID_GAP, 4, -1)).toBe(1 / 2)
    // the narrowest can't go under MIN_LINE_GAP: 20 / 4 = 5px
    expect(adaptiveStep(GRID_GAP, 4, -2)).toBe(1 / 2)
  })

  it('jumps from a beat to a bar in 3/4', () => {
    // 6px a beat: a beat is too fine (6px), a bar is 18px
    expect(adaptiveStep(6, 3)).toBe(3)
  })

  it('falls back to the coarsest step without a zoom', () => {
    expect(adaptiveStep(0, 4)).toBe(512 * 4)
    expect(adaptiveStep(NaN, 4)).toBe(512 * 4)
  })
})

describe('lineLevel', () => {
  it('is strongest on bars, then beats', () => {
    expect(lineLevel(0, 4)).toBe(2)
    expect(lineLevel(8, 4)).toBe(2)
    expect(lineLevel(5, 4)).toBe(1)
    expect(lineLevel(5.25, 4)).toBe(0)
    expect(lineLevel(6, 3)).toBe(2)
    expect(lineLevel(4, 3)).toBe(1)
  })
})

describe('tileStep', () => {
  it('shades a few lines at a time, on beats, bars or groups of steps', () => {
    expect(tileStep(1 / 4, 4)).toBe(1)
    expect(tileStep(1 / 16, 4)).toBe(1 / 4)
    expect(tileStep(1 / 2, 4)).toBe(1)
    expect(tileStep(1, 4)).toBe(4)
    expect(tileStep(2, 4)).toBe(4)
    expect(tileStep(4, 4)).toBe(16)
    expect(tileStep(1, 3)).toBe(3)
  })

  it('is always a whole number of steps', () => {
    gridStepOptions(4).forEach(step => {
      expect(Number.isInteger(tileStep(step, 4) / step)).toBe(true)
    })
  })
})

describe('stepLabel', () => {
  it('names notes and bars', () => {
    expect(stepLabel(1 / 4, 4)).toBe('1/16')
    expect(stepLabel(1, 4)).toBe('1/4')
    expect(stepLabel(2, 4)).toBe('1/2')
    expect(stepLabel(4, 4)).toBe('1 Bar')
    expect(stepLabel(16, 4)).toBe('4 Bars')
    expect(stepLabel(3, 3)).toBe('1 Bar')
    expect(stepLabel(1 / 64, 4)).toBe('1/256')
  })
})
