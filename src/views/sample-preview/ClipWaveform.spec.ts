import { centeredLevels } from './ClipWaveform'

describe('the sample editor’s waveform', () => {
  it('is at the sample’s real level, not stretched to fill the view', () => {
    const levels = centeredLevels(new Float32Array([0.5, 0.55, 0.45, 1, 0]))
    expect(levels.map(l => +l.toFixed(3))).toEqual([0, 0.1, -0.1, 1, -1])
  })
})
