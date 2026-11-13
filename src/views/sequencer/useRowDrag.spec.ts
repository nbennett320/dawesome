import { describe, expect, it } from 'vitest'
import { gapAt, moveIndex } from './useRowDrag'

describe('dragging a row', () => {
  it('finds the gap nearest the mouse', () => {
    // rows centred at 10, 40 and 70
    const mids = [10, 40, 70]
    expect(gapAt(mids, 0)).toBe(0)
    expect(gapAt(mids, 11)).toBe(1)
    expect(gapAt(mids, 39)).toBe(1)
    expect(gapAt(mids, 55)).toBe(2)
    expect(gapAt(mids, 90)).toBe(3)
  })

  it('counts where it goes without the row itself', () => {
    // the second of three rows
    expect(moveIndex(1, 0)).toBe(0)
    expect(moveIndex(1, 3)).toBe(2)
    // just above or below itself: it stays
    expect(moveIndex(1, 1)).toBeNull()
    expect(moveIndex(1, 2)).toBeNull()
    expect(moveIndex(0, 2)).toBe(1)
  })
})
