import {
  groupLanding, keepExisting, lassoRect, lassoSelection, rectsTouch, toggleSelected, ShiftPress, lassoFrom, shiftClick, startsLasso,
} from './selection'

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

describe('groupLanding', () => {
  const placed = [
    { id: 1, beat: 2, track: 0 },
    { id: 2, beat: 2.25, track: 1 },
    { id: 3, beat: 6, track: 1 },
  ]

  it('moves every clip by as much as the grabbed one', () => {
    expect(groupLanding(placed, 2, 4.25, 2, 16, 4)).toEqual([
      { id: 1, beat: 4, track: 1 },
      { id: 2, beat: 4.25, track: 2 },
      { id: 3, beat: 8, track: 2 },
    ])
  })

  it('stops the group at the start and end of the playlist', () => {
    expect(groupLanding(placed, 3, 3, 1, 16, 4).map(c => c.beat)).toEqual([0, 0.25, 4])
    expect(groupLanding(placed, 1, 15, 0, 16, 4).map(c => c.beat)).toEqual([12, 12.25, 16])
  })

  it('stops the group at the first track and the new-track area', () => {
    expect(groupLanding(placed, 2, 2.25, 0, 16, 4).map(c => c.track)).toEqual([0, 1, 1])
    expect(groupLanding(placed, 1, 2, 9, 16, 4).map(c => c.track)).toEqual([3, 4, 4])
  })

  it('lands nothing without the grabbed clip', () => {
    expect(groupLanding(placed, 9, 0, 0, 16, 4)).toEqual([])
  })
})

describe('a shift press', () => {
  const clips = [
    { id: 1, box: { left: 0, top: 24, width: 100, height: 100 } },
    { id: 2, box: { left: 300, top: 124, width: 100, height: 100 } },
  ]
  // pressed in the middle of clip 1
  const onClip: ShiftPress = { screen: { x: 50, y: 60 }, point: { x: 50, y: 60 }, clipId: 1 }
  const onEmpty: ShiftPress = { screen: { x: 200, y: 60 }, point: { x: 200, y: 60 }, clipId: null }

  it('is a click until the mouse has moved past the drag distance', () => {
    expect(startsLasso(onClip, { x: 50, y: 60 }, 10)).toBe(false)
    expect(startsLasso(onClip, { x: 59, y: 60 }, 10)).toBe(false)
    expect(startsLasso(onClip, { x: 50, y: 70 }, 10)).toBe(false)
    expect(startsLasso(onClip, { x: 61, y: 60 }, 10)).toBe(true)
    expect(startsLasso(onClip, { x: 42, y: 52 }, 10)).toBe(true)
  })

  it('let go without moving, toggles the clip under it', () => {
    expect([...shiftClick(onClip, new Set())]).toEqual([1])
    expect([...shiftClick(onClip, new Set([1, 2]))]).toEqual([2])
    // on empty space nothing changes
    expect([...shiftClick(onEmpty, new Set([2]))]).toEqual([2])
  })

  it('dragged from a clip, draws a box that takes that clip in', () => {
    const lasso = lassoFrom(onClip, new Set())
    expect(lasso.start).toEqual({ x: 50, y: 60 })
    // the box hasn't reached anything else yet
    expect([...lassoSelection({ ...lasso, end: { x: 62, y: 60 } }, clips)]).toEqual([1])
    // and once it has
    expect([...lassoSelection({ ...lasso, end: { x: 320, y: 130 } }, clips)].sort()).toEqual([1, 2])
  })

  it('adds to what was selected', () => {
    const lasso = lassoFrom(onEmpty, new Set([1]))
    expect([...lassoSelection({ ...lasso, end: { x: 320, y: 130 } }, clips)].sort()).toEqual([1, 2])
    // and never takes a clip out, even one the press was on
    const fromSelected = lassoFrom(onClip, new Set([1]))
    expect([...lassoSelection({ ...fromSelected, end: { x: 70, y: 60 } }, clips)]).toEqual([1])
  })
})
