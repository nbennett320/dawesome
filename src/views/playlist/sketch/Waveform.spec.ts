import { gainedLevel } from './Waveform'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

describe('a clip’s waveform at its gain', () => {
  it('is as recorded at 0 dB', () => {
    expect(gainedLevel(0.7, 1)).toBeCloseTo(0.7)
    expect(gainedLevel(0.2, 1)).toBeCloseTo(0.2)
  })

  it('grows away from the middle as the gain goes up, and shrinks as it goes down', () => {
    expect(gainedLevel(0.6, 2)).toBeCloseTo(0.7)
    expect(gainedLevel(0.4, 2)).toBeCloseTo(0.3)
    expect(gainedLevel(0.7, 0.5)).toBeCloseTo(0.6)
    // silence stays silent
    expect(gainedLevel(0.5, 4)).toBe(0.5)
  })

  it('is flat at the clip’s edges past full scale, as the sound clips', () => {
    expect(gainedLevel(0.9, 4)).toBe(1)
    expect(gainedLevel(0.1, 4)).toBe(0)
  })
})
