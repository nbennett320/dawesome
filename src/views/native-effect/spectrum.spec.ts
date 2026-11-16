import { FALL, FLOOR, follow, showing } from './spectrum'

const got = (written: number, levels: number[]) => ({ written, rate: 48000, levels })

describe('an analyzer’s picture', () => {
  it('starts as the first spectrum', () => {
    expect(follow(null, got(512, [-10, -50]))).toEqual({ levels: [-10, -50], written: 512 })
  })

  it('rises at once and falls slowly', () => {
    const first = follow(null, got(512, [-10, -50]))
    expect(follow(first, got(1024, [-40, -20]))!.levels).toEqual([-10 - FALL, -20])
  })

  it('keeps falling, to the floor, while nothing new comes', () => {
    let picture = follow(null, got(512, [-10, FLOOR]))
    // the effect stopped being run: the same frames each time
    picture = follow(picture, got(512, [-10, FLOOR]))
    expect(picture!.levels).toEqual([-10 - FALL, FLOOR])
    for(let i = 0; i < 200; i += 1) picture = follow(picture, got(512, [-10, FLOOR]))
    expect(picture!.levels).toEqual([FLOOR, FLOOR])
    expect(showing(picture)).toBe(false)
  })

  it('stays as it is when the engine says to ask again', () => {
    const picture = follow(null, got(512, [-10]))
    expect(follow(picture, null)).toBe(picture)
    expect(showing(picture)).toBe(true)
    expect(showing(null)).toBe(false)
  })
})
