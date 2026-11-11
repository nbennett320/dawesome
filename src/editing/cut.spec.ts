import { CutItem, canCut, cutPoint, cutTargets } from './cut'
import { Snapping } from './snap'

// the shortest a sample clip can be (10ms) at 120bpm, in beats
const least = 0.01 / 0.5
// a clip from beat 2 to beat 6
const clip: CutItem = { id: 1, start: 2, end: 6, minLength: least }
const free: Snapping = { enabled: false, every: 1, playhead: null, bypass: false }
const beats: Snapping = { ...free, enabled: true }

describe('cutPoint', () => {
  it('is where the mouse is with snap off', () => {
    expect(cutPoint(clip, 3.37, free)).toBe(3.37)
  })

  it('is on the grid with snap on, and where the mouse is with Option held', () => {
    expect(cutPoint(clip, 3.37, beats)).toBe(3)
    expect(cutPoint(clip, 3.6, beats)).toBe(4)
    expect(cutPoint(clip, 3.37, { ...beats, bypass: true })).toBe(3.37)
    expect(cutPoint(clip, 3.37, { ...beats, every: 0.25 })).toBe(3.25)
  })

  it('is the playhead when the mouse is nearer to it than to the grid, and it is not moving', () => {
    const stopped = { ...beats, playhead: 3.3 }
    expect(cutPoint(clip, 3.35, stopped)).toBe(3.3)
    expect(cutPoint(clip, 3.9, stopped)).toBe(4)
    // playing, the playhead is nowhere to snap to
    expect(cutPoint(clip, 3.35, { ...stopped, playhead: null })).toBe(3)
  })

  it('is nowhere within 10ms of either edge, so no cut is made and no line shown', () => {
    // 10ms is 0.02 beats here
    expect(least).toBeCloseTo(0.02, 10)
    expect(cutPoint(clip, 2.019, free)).toBeNull()
    expect(cutPoint(clip, 2.021, free)).toBe(2.021)
    expect(cutPoint(clip, 5.981, free)).toBeNull()
    expect(cutPoint(clip, 5.979, free)).toBe(5.979)
    expect(cutPoint(clip, 2, free)).toBeNull()
    expect(cutPoint(clip, 6, free)).toBeNull()
  })

  it('is nowhere when the snapped place is outside the clip', () => {
    // the mouse is just inside the clip, the nearest line is its own edge
    expect(cutPoint(clip, 2.2, beats)).toBeNull()
    // or a line before it: a clip that starts between lines
    const between = { ...clip, start: 2.4 }
    expect(cutPoint(between, 2.45, beats)).toBeNull()
    expect(cutPoint(between, 2.6, beats)).toBe(3)
    // a pattern clip can't be cut within a step of its edges
    const pattern: CutItem = { id: 2, start: 8, end: 12, minLength: 0.25 }
    expect(cutPoint(pattern, 8.2, free)).toBeNull()
    expect(cutPoint(pattern, 8.25, free)).toBe(8.25)
  })
})

describe('cutTargets', () => {
  const other: CutItem = { id: 2, start: 3, end: 9, minLength: least }
  const elsewhere: CutItem = { id: 3, start: 20, end: 24, minLength: least }
  const all = [clip, other, elsewhere]

  it('is the clip under the mouse, whether or not it is selected', () => {
    expect(cutTargets(all, 1, new Set(), 4)).toEqual([1])
    // other clips are selected, but not this one: only this one
    expect(cutTargets(all, 1, new Set([2, 3]), 4)).toEqual([1])
  })

  it('is every selected clip the cut is inside, when the clip is one of them', () => {
    expect(cutTargets(all, 1, new Set([1, 2, 3]), 4)).toEqual([1, 2])
    // an unselected clip under the same beat is left alone
    expect(cutTargets(all, 1, new Set([1, 3]), 4)).toEqual([1])
    // a selected clip the cut would leave a sliver of isn't cut
    expect(cutTargets(all, 1, new Set([1, 2]), 3.01)).toEqual([1])
  })

  it('is nothing when the clip under the mouse cannot be cut there', () => {
    expect(cutTargets(all, 1, new Set([1, 2]), 6)).toEqual([])
    expect(cutTargets(all, 9, new Set([1, 2]), 4)).toEqual([])
    expect(canCut(clip, 4)).toBe(true)
  })
})
