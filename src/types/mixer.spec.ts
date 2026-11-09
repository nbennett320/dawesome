import {
  dbToPosition, formatDb, formatPan, gainToDb, MAX_DB, MIN_DB, positionToDb,
} from './mixer'

describe('the fader scale', () => {
  it('runs from silent at the bottom to the loudest at the top', () => {
    expect(dbToPosition(MIN_DB)).toBe(0)
    expect(dbToPosition(-Infinity)).toBe(0)
    expect(dbToPosition(MAX_DB)).toBe(1)
    expect(dbToPosition(40)).toBe(1)
    expect(dbToPosition(0)).toBeCloseTo(0.8)
  })

  it('gives more room near 0 dB than at the quiet end', () => {
    const upper = dbToPosition(0) - dbToPosition(-10)
    const lower = dbToPosition(-50) - dbToPosition(-60)
    expect(upper).toBeGreaterThan(lower)
  })

  it('turns positions back into the same levels', () => {
    [-55, -40, -27.5, -12, -3, 0, 2.5, 6].forEach(db => {
      expect(positionToDb(dbToPosition(db))).toBeCloseTo(db)
    })
    expect(positionToDb(0)).toBe(MIN_DB)
    expect(positionToDb(-1)).toBe(MIN_DB)
    expect(positionToDb(2)).toBe(MAX_DB)
  })
})

describe('formatting', () => {
  it('shows levels in dB, with silence as -∞', () => {
    expect(formatDb(0)).toBe('0.0')
    expect(formatDb(-6.04)).toBe('-6.0')
    expect(formatDb(3.25)).toBe('+3.3')
    expect(formatDb(MIN_DB)).toBe('-∞')
    expect(formatDb(gainToDb(0))).toBe('-∞')
  })

  it('shows pan as centre, left or right', () => {
    expect(formatPan(0)).toBe('C')
    expect(formatPan(0.004)).toBe('C')
    expect(formatPan(-0.5)).toBe('L 50')
    expect(formatPan(1)).toBe('R 100')
  })

  it('reads meter levels as dB', () => {
    expect(gainToDb(1)).toBe(0)
    expect(gainToDb(0.1)).toBeCloseTo(-20)
  })
})
