import {
  clampClipSettings,
  defaultClipSettings,
  envelopeAt,
  gainFactor,
  isDefaultClipSettings,
  playbackRate,
  playedDuration,
} from './clip'

const settings = (overrides = {}) => ({ ...defaultClipSettings, ...overrides })

describe('clip settings', () => {
  test('pitch changes the speed like a tape: an octave doubles or halves it', () => {
    expect(playbackRate(settings())).toBe(1)
    expect(playbackRate(settings({ pitch: 12 }))).toBeCloseTo(2)
    expect(playbackRate(settings({ pitch: -12 }))).toBeCloseTo(0.5)
    expect(playedDuration(settings({ pitch: 12 }), 4)).toBeCloseTo(2)
  })

  test('gain in decibels', () => {
    expect(gainFactor(0)).toBe(1)
    expect(gainFactor(6)).toBeCloseTo(1.995, 3)
    expect(gainFactor(-20)).toBeCloseTo(0.1)
  })

  test('clamping keeps settings in range, and fades within the clip', () => {
    expect(clampClipSettings(settings({ gainDb: 40, pan: -3, pitch: 99 })))
      .toEqual(settings({ gainDb: 12, pan: -1, pitch: 24 }))

    expect(clampClipSettings(settings({ attackMs: 900, decayMs: 300 }), 0.5))
      .toEqual(settings({ attackMs: 500, decayMs: 300 }))

    // a cleared number field falls back to the default
    expect(clampClipSettings(settings({ gainDb: NaN })).gainDb).toBe(0)
  })

  test('the envelope fades in over the attack and out over the decay', () => {
    const s = settings({ attackMs: 100, decayMs: 200 })

    expect(envelopeAt(s, 0, 1)).toBe(0)
    expect(envelopeAt(s, 0.05, 1)).toBeCloseTo(0.5)
    expect(envelopeAt(s, 0.5, 1)).toBe(1)
    expect(envelopeAt(s, 0.9, 1)).toBeCloseTo(0.5)
    expect(envelopeAt(s, 1, 1)).toBe(0)

    // no fades: full volume throughout
    expect(envelopeAt(settings(), 0, 1)).toBe(1)
    expect(envelopeAt(settings(), 1, 1)).toBe(1)
  })

  test('overlapping fades: the quieter one wins', () => {
    const s = settings({ attackMs: 1000, decayMs: 1000 })

    expect(envelopeAt(s, 0.5, 1)).toBeCloseTo(0.5)
  })

  test('knows when nothing has been changed', () => {
    expect(isDefaultClipSettings(settings())).toBe(true)
    expect(isDefaultClipSettings(settings({ pan: 0.1 }))).toBe(false)
  })
})
