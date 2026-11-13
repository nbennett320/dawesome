import { describe, expect, it } from 'vitest'
import { gapAt, indexAmongAll, moveIndex } from './useRowDrag'

describe('dragging a row', () => {
  it('finds the gap nearest the mouse', () => {
    // rows centerd at 10, 40 and 70
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

  it('moves a row among those shown, keeping the rows not shown where they are', () => {
    // rows 1, 3 and 4 shown, 2 and 5 not
    const all = [1, 2, 3, 4, 5]
    const shown = [1, 3, 4]
    // 4 to the top: before 1
    expect(indexAmongAll(all, shown, 4, 0)).toBe(0)
    // 1 below 3: before 4, so after 2 and 3
    expect(indexAmongAll(all, shown, 1, 1)).toBe(2)
    // 1 to the bottom of those shown: just after 4, before 5
    expect(indexAmongAll(all, shown, 1, 2)).toBe(3)
    // the only row shown stays among the rest
    expect(indexAmongAll([2, 1, 5], [1], 1, 0)).toBe(1)
  })
})
