import { pasteShift, repeatShift, spanOf } from './clipboard'

const items = [{ start: 4, end: 6 }, { start: 2, end: 3 }, { start: 5, end: 9 }]

describe('spanOf', () => {
  it('runs from the earliest start to the latest end', () => {
    expect(spanOf(items)).toEqual({ start: 2, end: 9 })
  })

  it('is nothing for no items', () => {
    expect(spanOf([])).toBeNull()
  })
})

describe('pasteShift', () => {
  it('puts the earliest item where the paste is', () => {
    expect(pasteShift(items, 10)).toBe(8)
    expect(pasteShift(items, 0)).toBe(-2)
    expect(pasteShift([], 10)).toBe(0)
  })
})

describe('repeatShift', () => {
  it('starts the copies where the originals end', () => {
    expect(repeatShift(items)).toBe(7)
    expect(repeatShift([{ start: 3, end: 4 }])).toBe(1)
    expect(repeatShift([])).toBe(0)
  })
})
