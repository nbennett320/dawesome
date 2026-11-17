import { blockOf, isEmptySpan, shiftTracks, startAt, withEdgeBeat, withEdgeTrack } from './timeSelection'

describe('the time selection', () => {
  it('starts empty, at a beat on a track', () => {
    const sel = startAt(8, 2)
    expect(blockOf(sel)).toEqual({ start: 8, end: 8, top: 2, bottom: 2 })
    expect(isEmptySpan(sel)).toBe(true)
  })

  it('grows and shrinks from its anchor, either way along time', () => {
    const sel = withEdgeBeat(startAt(8, 2), 12)
    expect(blockOf(sel)).toEqual({ start: 8, end: 12, top: 2, bottom: 2 })
    expect(isEmptySpan(sel)).toBe(false)
    // back past the anchor: it's before it now
    expect(blockOf(withEdgeBeat(sel, 6))).toEqual({ start: 6, end: 8, top: 2, bottom: 2 })
    expect(isEmptySpan(withEdgeBeat(sel, 8 + 1e-9))).toBe(true)
  })

  it('grows over the tracks above or below, kept to the tracks there are', () => {
    let sel = withEdgeTrack(withEdgeBeat(startAt(8, 2), 12), 1, 4)
    expect(blockOf(sel)).toMatchObject({ top: 2, bottom: 3 })
    sel = withEdgeTrack(sel, 1, 4)
    expect(sel.edgeTrack).toBe(3)
    // back up past the anchor's track
    sel = withEdgeTrack(withEdgeTrack(withEdgeTrack(sel, -1, 4), -1, 4), -1, 4)
    expect(blockOf(sel)).toMatchObject({ top: 0, bottom: 2 })
    expect(withEdgeTrack(sel, -1, 4).edgeTrack).toBe(0)
  })

  it('moves a whole block a track at a time, until it reaches the first or last', () => {
    const sel = withEdgeTrack(withEdgeBeat(startAt(8, 1), 12), 1, 4)
    const down = shiftTracks(sel, 1, 4)
    expect(down).toEqual({ anchorBeat: 8, edgeBeat: 12, anchorTrack: 2, edgeTrack: 3 })
    expect(shiftTracks(down!, 1, 4)).toBeNull()
    expect(blockOf(shiftTracks(sel, -1, 4)!)).toMatchObject({ top: 0, bottom: 1 })
    expect(shiftTracks(shiftTracks(sel, -1, 4)!, -1, 4)).toBeNull()
  })
})
