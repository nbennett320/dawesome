import { LIFT, liftAmount } from './lift'

describe('a dragged clip’s lift', () => {
  it('eases up over LIFT.ms, quickly at first', () => {
    expect(liftAmount(0, false)).toBe(0)
    expect(liftAmount(LIFT.ms / 2, false)).toBeGreaterThan(0.5)
    expect(liftAmount(LIFT.ms, false)).toBe(1)
    expect(liftAmount(LIFT.ms * 10, false)).toBe(1)
  })

  it('is up at once with reduced motion', () => {
    expect(liftAmount(0, true)).toBe(1)
  })
})
