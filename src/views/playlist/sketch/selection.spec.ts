import { keepExisting, lassoRect, lassoSelection, rectsTouch, toggleSelected } from './selection'

const rect = (left: number, top: number, width: number, height: number) => ({ left, top, width, height })

describe('lassoRect', () => {
  it('is the same box whichever way it was dragged', () => {
    const forward = lassoRect({ start: { x: 10, y: 20 }, end: { x: 50, y: 80 } })
    const back = lassoRect({ start: { x: 50, y: 80 }, end: { x: 10, y: 20 } })
    expect(forward).toEqual(rect(10, 20, 40, 60))
    expect(back).toEqual(forward)
  })
})

describe('rectsTouch', () => {
  const clip = rect(100, 100, 50, 40)

  it('picks up a clip the box overlaps partly', () => {
    expect(rectsTouch(rect(140, 130, 100, 100), clip)).toBe(true)
  })

  it('picks up a clip inside the box, and a box inside a clip', () => {
    expect(rectsTouch(rect(0, 0, 500, 500), clip)).toBe(true)
    expect(rectsTouch(rect(110, 110, 5, 5), clip)).toBe(true)
  })

  it('picks up a clip crossed by a box with no width', () => {
    expect(rectsTouch(rect(120, 0, 0, 300), clip)).toBe(true)
  })

  it('misses a clip beside or below the box', () => {
    expect(rectsTouch(rect(0, 100, 99, 40), clip)).toBe(false)
    expect(rectsTouch(rect(100, 141, 50, 40), clip)).toBe(false)
  })
})

describe('lassoSelection', () => {
  const clips = [
    { id: 1, box: rect(0, 0, 50, 40) },
    { id: 2, box: rect(100, 0, 50, 40) },
    { id: 3, box: rect(0, 50, 50, 40) },
  ]

  it('selects the clips the box touches', () => {
    const lasso = { start: { x: 40, y: 10 }, end: { x: 110, y: 20 }, base: new Set<number>() }
    expect([...lassoSelection(lasso, clips)].sort()).toEqual([1, 2])
  })

  it('keeps what was already selected', () => {
    const lasso = { start: { x: 120, y: 10 }, end: { x: 130, y: 20 }, base: new Set([3]) }
    expect([...lassoSelection(lasso, clips)].sort()).toEqual([2, 3])
  })

  it("doesn't change the selection it started from", () => {
    const base = new Set([3])
    lassoSelection({ start: { x: 0, y: 0 }, end: { x: 200, y: 200 }, base }, clips)
    expect([...base]).toEqual([3])
  })
})

describe('toggleSelected', () => {
  it('adds a clip that is not selected, and removes one that is', () => {
    expect([...toggleSelected(new Set([1]), 2)].sort()).toEqual([1, 2])
    expect([...toggleSelected(new Set([1, 2]), 2)]).toEqual([1])
  })
})

describe('keepExisting', () => {
  it('drops clips that are gone', () => {
    expect([...keepExisting(new Set([1, 2, 3]), [3, 1, 7])].sort()).toEqual([1, 3])
  })
})
