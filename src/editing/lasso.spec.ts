import {
  keepExisting, lassoRect, lassoSelection, rectsTouch, toggleSelected, ShiftPress, itemAt, lassoFrom,
  shiftClick, shiftRelease, startsLasso,
} from './lasso'

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

  it('replaces what was already selected', () => {
    const lasso = { start: { x: 120, y: 10 }, end: { x: 130, y: 20 }, base: new Set([3]) }
    expect([...lassoSelection(lasso, clips)]).toEqual([2])
  })

  it('keeps what was already selected while it touches nothing', () => {
    const lasso = { start: { x: 60, y: 10 }, end: { x: 90, y: 45 }, base: new Set([3]) }
    expect([...lassoSelection(lasso, clips)]).toEqual([3])
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

describe('a shift press', () => {
  const clips = [
    { id: 1, box: { left: 380, top: 224, width: 100, height: 100 } },
    { id: 2, box: { left: 680, top: 324, width: 100, height: 100 } },
  ]
  // the view is scrolled and zoomed, so where the press is on the canvas
  // (`screen`) isn't where it is in the playlist (`point`).
  // pressed in the middle of clip 1
  const onClip: ShiftPress = { screen: { x: 50, y: 60 }, point: { x: 430, y: 260 }, itemId: 1 }
  const onEmpty: ShiftPress = { screen: { x: 200, y: 60 }, point: { x: 580, y: 260 }, itemId: null }
  // the mouse `dx`, `dy` canvas pixels from where `press` went down (and,
  // zoomed in 2x, half as far in the playlist's x)
  const moved = (press: ShiftPress, dx: number, dy: number) => ({
    screen: { x: press.screen.x + dx, y: press.screen.y + dy },
    point: { x: press.point.x + dx / 2, y: press.point.y + dy },
  })

  it('finds the clip under a point, the top one where clips overlap', () => {
    const over = [...clips, { id: 3, box: { left: 400, top: 224, width: 50, height: 100 } }]
    expect(itemAt({ x: 430, y: 260 }, clips)).toBe(1)
    expect(itemAt({ x: 430, y: 260 }, over)).toBe(3)
    expect(itemAt({ x: 470, y: 260 }, over)).toBe(1)
    expect(itemAt({ x: 580, y: 260 }, over)).toBeNull()
    expect(itemAt({ x: 430, y: 200 }, over)).toBeNull()
  })

  it('is a click until the mouse has moved past the drag distance on the canvas', () => {
    expect(startsLasso(onClip, { x: 50, y: 60 }, 10)).toBe(false)
    expect(startsLasso(onClip, { x: 59, y: 60 }, 10)).toBe(false)
    expect(startsLasso(onClip, { x: 50, y: 70 }, 10)).toBe(false)
    expect(startsLasso(onClip, { x: 61, y: 60 }, 10)).toBe(true)
    expect(startsLasso(onClip, { x: 42, y: 52 }, 10)).toBe(true)
    // measured on the canvas, not in the playlist: the press's own playlist
    // point is far from its canvas point, and isn't a drag
    expect(startsLasso(onClip, onClip.point, 10)).toBe(true)
    expect(startsLasso(onClip, onClip.screen, 10)).toBe(false)
  })

  it('let go without moving, toggles the clip under it', () => {
    expect([...shiftClick(onClip, new Set())]).toEqual([1])
    expect([...shiftClick(onClip, new Set([1, 2]))]).toEqual([2])
    // on empty space nothing changes
    expect([...shiftClick(onEmpty, new Set([2]))]).toEqual([2])
  })

  it('dragged from a clip, draws a box that takes that clip in', () => {
    const lasso = lassoFrom(onClip, new Set())
    // the box starts where the press was in the playlist, however the view
    // is scrolled
    expect(lasso.start).toEqual({ x: 430, y: 260 })
    expect(lasso.end).toEqual({ x: 430, y: 260 })
    // the box hasn't reached anything else yet
    expect([...lassoSelection({ ...lasso, end: { x: 442, y: 260 } }, clips)]).toEqual([1])
    // and once it has
    expect([...lassoSelection({ ...lasso, end: { x: 700, y: 330 } }, clips)].sort()).toEqual([1, 2])
  })

  it('replaces what was selected once it touches a clip', () => {
    const lasso = lassoFrom(onEmpty, new Set([1]))
    // nothing touched yet: the selection stays
    expect([...lassoSelection({ ...lasso, end: { x: 600, y: 270 } }, clips)]).toEqual([1])
    // clip 2 touched: clip 1 drops out
    expect([...lassoSelection({ ...lasso, end: { x: 700, y: 330 } }, clips)]).toEqual([2])
    // a box from a selected clip keeps it, since the box touches it
    const fromSelected = lassoFrom(onClip, new Set([1, 2]))
    expect([...lassoSelection({ ...fromSelected, end: { x: 450, y: 260 } }, clips)]).toEqual([1])
  })

  describe('let go', () => {
    const release = (press: ShiftPress, dx: number, dy: number, selected: number[], existing = clips) =>
      [...shiftRelease(press, moved(press, dx, dy), 10, new Set(selected), existing)].sort()

    it('within the drag distance, is a click', () => {
      expect(release(onClip, 0, 0, [])).toEqual([1])
      expect(release(onClip, 9, 0, [1, 2])).toEqual([2])
      expect(release(onClip, 10, 0, [1])).toEqual([])
      expect(release(onEmpty, 3, 3, [2])).toEqual([2])
    })

    it('past the drag distance, is a box, even if no frame saw it start', () => {
      // from a selected clip: a click would take it out, a box keeps it
      expect(release(onClip, 11, 0, [1])).toEqual([1])
      expect(release(onClip, 11, 0, [])).toEqual([1])
      // and drops what it doesn't touch
      expect(release(onClip, 11, 0, [2])).toEqual([1])
      // dragged across to the second clip
      expect(release(onClip, 520, 70, [])).toEqual([1, 2])
      // from empty space, onto nothing
      expect(release(onEmpty, 20, 0, [2])).toEqual([2])
    })

    it('on a clip that has gone since the press, changes nothing', () => {
      // deleted, or exploded, while the mouse was down
      expect(release(onClip, 0, 0, [], [clips[1]])).toEqual([])
      expect(release(onClip, 0, 0, [2], [clips[1]])).toEqual([2])
    })
  })
})
