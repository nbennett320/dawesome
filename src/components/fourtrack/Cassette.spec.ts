// (rendering it isn't tested: with esModuleInterop off, React's default
// import is undefined under jest)
import {
  FULL_RADIUS,
  HUB_RADIUS,
  MAX_SPIN,
  packRadii,
  reelSpin,
} from './Cassette'

const area = ({ left, right }: { left: number, right: number }) => left ** 2 + right ** 2

describe('packRadii', () => {
  it('starts with all the tape on the left reel', () => {
    expect(packRadii(0)).toEqual({ left: FULL_RADIUS, right: HUB_RADIUS })
  })

  it('ends with all the tape on the right reel', () => {
    expect(packRadii(1)).toEqual({ left: HUB_RADIUS, right: FULL_RADIUS })
  })

  it('splits the tape evenly halfway', () => {
    const { left, right } = packRadii(0.5)
    expect(left).toBeCloseTo(right)
  })

  it('keeps the same amount of tape throughout', () => {
    const total = area(packRadii(0))
    ;[0.1, 0.25, 0.5, 0.9].forEach(p => expect(area(packRadii(p))).toBeCloseTo(total))
  })

  it('keeps progress within the tape', () => {
    expect(packRadii(-1)).toEqual(packRadii(0))
    expect(packRadii(2)).toEqual(packRadii(1))
    expect(packRadii(NaN)).toEqual(packRadii(0))
  })
})

describe('reelSpin', () => {
  it('is still when the tape is', () => {
    expect(reelSpin(20, 0)).toBe(0)
  })

  it('turns a small pack faster than a big one', () => {
    expect(Math.abs(reelSpin(HUB_RADIUS, 1))).toBeGreaterThan(Math.abs(reelSpin(FULL_RADIUS, 1)))
  })

  it('turns anticlockwise going forward and clockwise winding back', () => {
    expect(reelSpin(20, 1)).toBeLessThan(0)
    expect(reelSpin(20, -1)).toBeGreaterThan(0)
  })

  it('never turns fast enough to look like it is going backwards', () => {
    expect(Math.abs(reelSpin(HUB_RADIUS, 12))).toBe(MAX_SPIN)
    expect(Math.abs(reelSpin(HUB_RADIUS, -12))).toBe(MAX_SPIN)
  })
})

