import { blockOf, isEmptySpan, shiftTracks, bladePlan, startAt, withEdgeBeat, withEdgeTrack } from './timeSelection'

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

describe('bladePlan', () => {
  // clips 1 (beats 0–8) and 2 (8–16) on track 0, 3 (4–12) on track 1, 4
  // (0–16) on track 3
  const clip = (id: number, track: number, start: number, end: number) =>
    ({ track, clip: { id, start, end, minLength: 0.25 } })
  const clips = [clip(1, 0, 0, 8), clip(2, 0, 8, 16), clip(3, 1, 4, 12), clip(4, 3, 0, 16)]

  it("cuts the current track's clip under the playhead", () => {
    expect(bladePlan(null, 6, 0, clips)).toEqual({ ids: [1], beats: [6] })
    expect(bladePlan(null, 6, 1, clips)).toEqual({ ids: [3], beats: [6] })
    // on an edge, or nothing there
    expect(bladePlan(null, 8, 0, clips)).toBeNull()
    expect(bladePlan(null, 6, 2, clips)).toBeNull()
    expect(bladePlan(null, null, 0, clips)).toBeNull()
  })

  it("cuts the selection's clips at both its ends, on its tracks only", () => {
    const sel = withEdgeTrack(withEdgeBeat(startAt(6, 0), 10), 1, 4)
    expect(bladePlan(sel, 0, 1, clips)).toEqual({ ids: [1, 2, 3], beats: [6, 10] })
  })

  it('cuts at just the one beat while the selection covers no time', () => {
    const sel = withEdgeTrack(withEdgeTrack(startAt(6, 1), 1, 4), 1, 4)
    expect(bladePlan(sel, 0, 3, clips)).toEqual({ ids: [3, 4], beats: [6] })
  })
})
