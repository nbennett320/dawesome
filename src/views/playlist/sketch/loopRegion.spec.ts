import {
  LoopMarks, doubleClickClears, draggedMarks, draggedRegion, handleAt, loopChange, loopMarks, moveLoopDrag,
  startLoopDrag,
} from './loopRegion'

const transport = (t: Partial<Parameters<typeof loopMarks>[0] & object> = {}) => ({
  loopStartBeats: 0, loopEndBeats: 16, loopRegion: false, loopEnabled: true, ...t,
})
const region = (start: number, end: number): LoopMarks => ({ region: { start, end }, flag: null })
const flag = (at: number): LoopMarks => ({ region: null, flag: at })

describe('the loop in the ruler', () => {
  it('is the region, or the end flag while looping', () => {
    expect(loopMarks(transport({ loopStartBeats: 4, loopEndBeats: 8, loopRegion: true }))).toEqual(region(4, 8))
    // a region shows with looping off too
    expect(loopMarks(transport({ loopStartBeats: 4, loopEndBeats: 8, loopRegion: true, loopEnabled: false })))
      .toEqual(region(4, 8))
    expect(loopMarks(transport())).toEqual(flag(16))
    expect(loopMarks(transport({ loopEnabled: false }))).toEqual({ region: null, flag: null })
    expect(loopMarks(null)).toEqual({ region: null, flag: null })
  })
})

describe('handleAt', () => {
  it('grabs an edge of the region near the press', () => {
    expect(handleAt(region(4, 8), 4.1, 0.25)).toBe('start')
    expect(handleAt(region(4, 8), 7.8, 0.25)).toBe('end')
    expect(handleAt(region(4, 8), 8.2, 0.25)).toBe('end')
    // nowhere near either: a press in the ruler seeks
    expect(handleAt(region(4, 8), 6, 0.25)).toBeNull()
  })

  it('takes the nearer edge of a very short region', () => {
    expect(handleAt(region(4, 4.25), 4.05, 0.5)).toBe('start')
    expect(handleAt(region(4, 4.25), 4.2, 0.5)).toBe('end')
    // equally near both: the side of the middle it's on
    expect(handleAt(region(4, 4.25), 4.125, 0.5)).toBe('end')
    expect(handleAt(region(4, 4), 3.9, 0.5)).toBe('start')
    expect(handleAt(region(4, 4), 4.1, 0.5)).toBe('end')
  })

  it('grabs the end flag when there is no region', () => {
    expect(handleAt(flag(16), 15.9, 0.25)).toBe('flag')
    expect(handleAt(flag(16), 15, 0.25)).toBeNull()
    expect(handleAt({ region: null, flag: null }, 16, 0.25)).toBeNull()
  })
})

describe('dragging in the ruler', () => {
  it('moves one edge, keeping the other', () => {
    let drag = startLoopDrag('end', region(4, 8), 8)
    drag = moveLoopDrag(drag, 10)
    expect(draggedRegion(drag)).toEqual({ start: 4, end: 10 })
    expect(loopChange(drag)).toEqual({ kind: 'region', start: 4, end: 10 })
  })

  it('swaps the edges when one is dragged past the other', () => {
    const drag = moveLoopDrag(startLoopDrag('start', region(4, 8), 4), 11)
    expect(draggedRegion(drag)).toEqual({ start: 8, end: 11 })
    expect(draggedMarks(region(4, 8), drag)).toEqual(region(8, 11))
  })

  it('has no region with both edges on one beat', () => {
    const drag = moveLoopDrag(startLoopDrag('start', region(4, 8), 4), 8)
    expect(draggedRegion(drag)).toBeNull()
    expect(draggedMarks(region(4, 8), drag)).toEqual({ region: null, flag: null })
    // the engine is asked for the same beat twice, which clears it
    expect(loopChange(drag)).toEqual({ kind: 'region', start: 8, end: 8 })
  })

  it('draws a new region from the press, in either direction', () => {
    const drag = moveLoopDrag(startLoopDrag('draw', flag(16), 6), 2)
    expect(draggedRegion(drag)).toEqual({ start: 2, end: 6 })
    expect(loopChange(drag)).toEqual({ kind: 'region', start: 6, end: 2 })
  })

  it('clears the region with a shift+click that does not move', () => {
    const drag = startLoopDrag('draw', region(4, 8), 6)
    expect(draggedMarks(region(4, 8), drag)).toEqual({ region: null, flag: null })
    expect(loopChange(drag)).toEqual({ kind: 'region', start: 6, end: 6 })
  })

  it('pins the end flag where it is dropped, at least a beat in', () => {
    let drag = moveLoopDrag(startLoopDrag('flag', flag(16), 16), 12)
    expect(draggedMarks(flag(16), drag)).toEqual(flag(12))
    expect(loopChange(drag)).toEqual({ kind: 'end', beat: 12 })
    drag = moveLoopDrag(drag, 0)
    expect(drag.beat).toBe(1)
  })

  it('changes nothing when an edge or the flag is let go where it was', () => {
    expect(loopChange(startLoopDrag('flag', flag(16), 16))).toBeNull()
    expect(loopChange(moveLoopDrag(startLoopDrag('end', region(4, 8), 8), 8))).toBeNull()
  })
})

describe('double-clicking in the ruler', () => {
  it('clears the region when on it', () => {
    expect(doubleClickClears(region(4, 8), false, 6, 0.25)).toBe('region')
    expect(doubleClickClears(region(4, 8), false, 8.1, 0.25)).toBe('region')
    expect(doubleClickClears(region(4, 8), false, 10, 0.25)).toBeNull()
  })

  it('unpins a dragged end flag when on it', () => {
    expect(doubleClickClears(flag(12), true, 12.1, 0.25)).toBe('end')
    // one following the last clip has nothing to undo
    expect(doubleClickClears(flag(12), false, 12, 0.25)).toBeNull()
    expect(doubleClickClears(flag(12), true, 6, 0.25)).toBeNull()
  })
})
