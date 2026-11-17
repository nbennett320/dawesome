import { LANDING, flashStrength, glideBeat, glideProgress } from './landing'

describe('the glide to a landmark', () => {
  it('eases out, from the start to the end', () => {
    expect(glideProgress(0)).toBe(0)
    expect(glideProgress(LANDING.glideMs / 2)).toBeCloseTo(0.875)
    expect(glideProgress(LANDING.glideMs)).toBe(1)
    expect(glideProgress(LANDING.glideMs * 3)).toBe(1)
    expect(glideProgress(-5)).toBe(0)
  })

  it('draws the playhead part way there, either way', () => {
    expect(glideBeat(16, 0, 0)).toBe(16)
    expect(glideBeat(16, 0, LANDING.glideMs / 2)).toBeCloseTo(2)
    expect(glideBeat(4, 12, LANDING.glideMs)).toBe(12)
  })
})

describe('the flash where it lands', () => {
  it('starts bright and fades out', () => {
    expect(flashStrength(0)).toBe(1)
    expect(flashStrength(LANDING.flashMs / 2)).toBeCloseTo(0.25)
    expect(flashStrength(LANDING.flashMs)).toBe(0)
  })
})
