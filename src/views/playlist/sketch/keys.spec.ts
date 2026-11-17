// the playlist's keys, on a renderer with no canvas: moving the playhead,
// the current track, and the time selection
import { Renderer } from './index'
import { PlaylistTransport } from '../../../types/transport'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const stopped: PlaylistTransport = {
  playing: false, startTimeMs: null, startOffsetMs: 0, positionMs: 0, lastPlayStartBeats: null, beatIntervalMs: 500,
  loopStartBeats: 0, loopEndBeats: 16, loopRegion: false, loopEndPinned: false,
  beatsDisplayed: 384, loopEnabled: true, songEndMs: 8_000, beatsPerBar: 4, patternId: null, patternLength: 0,
}

// 4 tracks, stopped with the playhead at `beat`, snapped to beats
const renderer = (beat = 0) => {
  const r = new Renderer()
  r.width = 960
  r.height = 400
  r.beatsDisplayed = 384
  r.trackCount = 4
  r.transport = { ...stopped, positionMs: beat * 500, startOffsetMs: beat * 500 }
  const seeks: number[] = []
  r.onSeek = (b: number) => { seeks.push(b); return Promise.resolve() }
  const tracks: number[] = []
  r.onCurrentTrack = (t: number) => { tracks.push(t) }
  return { r, seeks, tracks }
}

const right = { kind: 'step', dir: 1 } as const
const left = { kind: 'step', dir: -1 } as const

describe('⇧ and the arrows', () => {
  it('select from the playhead, which follows the edge while stopped', () => {
    const { r, seeks } = renderer(4)
    r.navigate(right, true)
    r.navigate(right, true)
    expect(r.timeSelection).toEqual({ anchorBeat: 4, edgeBeat: 6, anchorTrack: 0, edgeTrack: 0 })
    expect(seeks).toEqual([5, 6])
    // back past the anchor
    r.navigate(left, true)
    r.navigate(left, true)
    r.navigate(left, true)
    expect(r.selectedSpan()).toEqual({ start: 3, end: 4 })
  })

  it('move the edge on its own while playing, leaving playback alone', () => {
    const { r, seeks } = renderer(4)
    r.playing = true
    // (the playhead, part way through beat 6)
    r.seekBeat = 6.4
    r.navigate(right, true)
    expect(r.timeSelection).toMatchObject({ anchorBeat: 6, edgeBeat: 7 })
    r.seekBeat = 9.2
    r.navigate(right, true)
    expect(r.timeSelection).toMatchObject({ anchorBeat: 6, edgeBeat: 8 })
    expect(seeks).toEqual([])
  })

  it('jump to landmarks from the edge', () => {
    const { r } = renderer(4)
    r.navigate({ kind: 'landmark', dir: 1 }, true)
    // the end flag (following the last clip, at beat 16)
    expect(r.selectedSpan()).toEqual({ start: 4, end: 16 })
  })

  it('clear the clips selected, and a plain move clears the time selection', () => {
    const { r } = renderer(4)
    r.select([1, 2])
    r.navigate(right, true)
    expect(r.selectedIds.size).toBe(0)
    r.navigate(right)
    expect(r.timeSelection).toBeNull()
  })

  it('do nothing but with the Select tool', () => {
    const { r, seeks } = renderer(4)
    r.setTool('draw')
    expect(r.navigate(right, true)).toBeNull()
    r.stepTrack(1, true)
    expect(r.timeSelection).toBeNull()
    expect(seeks).toEqual([])
  })
})

describe('selecting clips', () => {
  it('clears the time selection', () => {
    const { r } = renderer(4)
    r.navigate(right, true)
    r.select([3])
    expect(r.timeSelection).toBeNull()
    // (an empty one leaves it)
    r.navigate(right, true)
    r.clearSelection()
    expect(r.timeSelection).not.toBeNull()
  })
})

describe('↑ and ↓', () => {
  it('move the current track, stopping at the first and last', () => {
    const { r, tracks } = renderer()
    r.stepTrack(1)
    r.stepTrack(1)
    r.stepTrack(-1)
    expect(tracks).toEqual([1, 2, 1])
    r.stepTrack(-1)
    r.stepTrack(-1)
    expect(r.currentTrack).toBe(0)
    expect(tracks).toEqual([1, 2, 1, 0])
  })

  it('with ⇧, grow the selection over tracks, as a block', () => {
    const { r } = renderer(4)
    r.setCurrentTrack(1)
    r.navigate(right, true)
    r.stepTrack(1, true)
    r.stepTrack(1, true)
    expect(r.timeSelection).toEqual({ anchorBeat: 4, edgeBeat: 5, anchorTrack: 1, edgeTrack: 3 })
    expect(r.currentTrack).toBe(3)
  })

  it('with ⇧ and no selection, start one over the tracks at the playhead', () => {
    const { r } = renderer(4)
    r.stepTrack(1, true)
    expect(r.timeSelection).toEqual({ anchorBeat: 4, edgeBeat: 4, anchorTrack: 0, edgeTrack: 1 })
    // which covers no time, so there's nothing to loop yet
    expect(r.selectedSpan()).toBeNull()
  })

  it('move a block whole, keeping it when it reaches the first or last track', () => {
    const { r } = renderer(4)
    r.navigate(right, true)
    r.stepTrack(1, true)
    r.stepTrack(1)
    expect(r.timeSelection).toEqual({ anchorBeat: 4, edgeBeat: 5, anchorTrack: 1, edgeTrack: 2 })
    r.stepTrack(1)
    r.stepTrack(1)
    expect(r.timeSelection).toMatchObject({ anchorTrack: 2, edgeTrack: 3 })
    expect(r.currentTrack).toBe(3)
    r.stepTrack(-1)
    r.stepTrack(-1)
    r.stepTrack(-1)
    expect(r.timeSelection).toMatchObject({ anchorTrack: 0, edgeTrack: 1 })
    expect(r.currentTrack).toBe(1)
  })
})

describe('another track', () => {
  it('pressed or landed on, clears the time selection', () => {
    const { r } = renderer(4)
    r.navigate(right, true)
    // the edge's own track keeps it
    r.touchTrack(0)
    expect(r.timeSelection).not.toBeNull()
    r.touchTrack(2)
    expect(r.timeSelection).toBeNull()
  })
})

describe('a jump to a landmark', () => {
  let now = 1_000
  beforeEach(() => {
    now = 1_000
    vi.spyOn(performance, 'now').mockImplementation(() => now)
  })
  afterEach(() => vi.restoreAllMocks())

  it('glides the playhead there, and flashes where it lands', () => {
    const { r } = renderer(16)
    r.navigate({ kind: 'home' })
    // the engine's playhead is there at once; the one drawn glides
    expect(r.playheadBeat()).toBe(0)
    expect(r.shownPlayheadBeat()).toBe(16)
    now += 60
    expect(r.shownPlayheadBeat()).toBeCloseTo(2)
    expect(r.landmarkFlash()).toMatchObject({ beat: 0 })
    now += 1_000
    expect(r.shownPlayheadBeat()).toBe(0)
    expect(r.landmarkFlash()?.strength).toBe(0)
  })

  it("doesn't with experimental animations off", () => {
    const { r } = renderer(16)
    r.experimentalAnimations = false
    r.navigate({ kind: 'home' })
    expect(r.shownPlayheadBeat()).toBe(0)
    expect(r.landmarkFlash()).toBeNull()
  })

  it('is cut short by a step, which is instant', () => {
    const { r } = renderer(16)
    r.navigate({ kind: 'home' })
    r.navigate(right)
    expect(r.shownPlayheadBeat()).toBe(1)
  })

  it("flashes, but doesn't glide, when only a selection's edge moved", () => {
    const { r } = renderer(4)
    r.playing = true
    r.seekBeat = 4
    r.navigate({ kind: 'landmark', dir: 1 }, true)
    expect(r.shownPlayheadBeat()).toBe(4)
    expect(r.landmarkFlash()).toMatchObject({ beat: 16 })
  })
})
