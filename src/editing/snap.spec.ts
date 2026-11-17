import { SNAP_OPTIONS, snapLabel, stepSnapNote } from './snap'

describe('the snap lengths', () => {
  it('go from a bar down to a quarter beat', () => {
    expect(SNAP_OPTIONS.map(option => option.label)).toEqual(['1 bar', '1/2 bar', '1 beat', '1/2 beat', '1/4 beat'])
    expect(snapLabel('EighthNote')).toBe('1/2 beat')
  })

  it('step finer or coarser, stopping at either end', () => {
    expect(stepSnapNote('QuarterNote', 1)).toBe('EighthNote')
    expect(stepSnapNote('QuarterNote', -1)).toBe('HalfNote')
    expect(stepSnapNote('SixteenthNote', 1)).toBe('SixteenthNote')
    expect(stepSnapNote('WholeNote', -1)).toBe('WholeNote')
  })
})
