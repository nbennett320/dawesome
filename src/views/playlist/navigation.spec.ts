import {
  Landmarks, Move, Place, landmarkNote, landmarks, moveForKey, positionLabel, moveTarget, nextLandmark, stepFrom, trackStepForKey,
} from './navigation'

const key = (k: string, mods: Partial<Record<'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey', boolean>> = {}) => ({
  key: k, shiftKey: false, metaKey: false, ctrlKey: false, altKey: false, ...mods,
})

describe('moveForKey', () => {
  it('reads the arrows as a text cursor does, on a Mac', () => {
    expect(moveForKey(key('ArrowLeft'), true)).toEqual({ kind: 'step', dir: -1 })
    expect(moveForKey(key('ArrowRight'), true)).toEqual({ kind: 'step', dir: 1 })
    expect(moveForKey(key('ArrowLeft', { altKey: true }), true)).toEqual({ kind: 'bar', dir: -1 })
    expect(moveForKey(key('ArrowRight', { metaKey: true }), true)).toEqual({ kind: 'landmark', dir: 1 })
    expect(moveForKey(key('Home'), true)).toEqual({ kind: 'home' })
    expect(moveForKey(key('End'), true)).toEqual({ kind: 'end' })
  })

  it('takes Ctrl for ⌘ on other systems', () => {
    expect(moveForKey(key('ArrowLeft', { ctrlKey: true }), false)).toEqual({ kind: 'landmark', dir: -1 })
    expect(moveForKey(key('ArrowLeft', { metaKey: true }), false)).toBeNull()
    // ⌃← on a Mac belongs to the system (it switches desktops)
    expect(moveForKey(key('ArrowLeft', { ctrlKey: true }), true)).toBeNull()
  })

  it('reads them the same with Shift held (a selection), and leaves other keys and mixes alone', () => {
    expect(moveForKey(key('ArrowLeft', { shiftKey: true }), true)).toEqual({ kind: 'step', dir: -1 })
    expect(moveForKey(key('ArrowRight', { shiftKey: true, metaKey: true }), true)).toEqual({ kind: 'landmark', dir: 1 })
    expect(moveForKey(key('ArrowUp'), true)).toBeNull()
    expect(moveForKey(key('ArrowLeft', { metaKey: true, altKey: true }), true)).toBeNull()
    expect(moveForKey(key('Home', { metaKey: true }), true)).toBeNull()
  })
})

const transport = {
  loopStartBeats: 0, loopEndBeats: 16, loopRegion: false, loopEnabled: true,
  songEndMs: 6_000, beatIntervalMs: 500, lastPlayStartBeats: null,
}

describe('landmarks', () => {
  it('has the end flag as the loop end while looping, with no region', () => {
    expect(landmarks(transport)).toEqual({ lastPlayStart: null, loopStart: null, loopEnd: 16, songEnd: 12 })
    // looping off: the flag isn't shown, so isn't a landmark
    expect(landmarks({ ...transport, loopEnabled: false }).loopEnd).toBeNull()
  })

  it("has a region's edges, shown with looping off too", () => {
    const t = { ...transport, loopRegion: true, loopStartBeats: 4, loopEndBeats: 8, lastPlayStartBeats: 2 }
    expect(landmarks(t)).toEqual({ lastPlayStart: 2, loopStart: 4, loopEnd: 8, songEnd: 12 })
    expect(landmarks({ ...t, loopEnabled: false })).toMatchObject({ loopStart: 4, loopEnd: 8 })
  })
})

const marks = (m: Partial<Landmarks> = {}): Landmarks => ({
  lastPlayStart: null, loopStart: null, loopEnd: null, songEnd: 32, ...m,
})

describe('nextLandmark', () => {
  it('climbs ⌘← from the last play start to the loop start to the song start', () => {
    const m = marks({ lastPlayStart: 20, loopStart: 8 })
    expect(nextLandmark(m, 26, -1)).toEqual({ name: 'last-play-start', beat: 20 })
    expect(nextLandmark(m, 20, -1)).toEqual({ name: 'loop-start', beat: 8 })
    expect(nextLandmark(m, 8, -1)).toEqual({ name: 'song-start', beat: 0 })
    expect(nextLandmark(m, 0, -1)).toBeNull()
  })

  it('passes over landmarks that are missing, or the other way', () => {
    // nothing's played and there's no region
    expect(nextLandmark(marks(), 12, -1)).toEqual({ name: 'song-start', beat: 0 })
    // the last play start is after the playhead
    expect(nextLandmark(marks({ lastPlayStart: 20, loopStart: 8 }), 12, -1)).toEqual({ name: 'loop-start', beat: 8 })
  })

  it('climbs ⌘→ from the loop end to the song end', () => {
    const m = marks({ loopEnd: 16 })
    expect(nextLandmark(m, 4, 1)).toEqual({ name: 'loop-end', beat: 16 })
    expect(nextLandmark(m, 16, 1)).toEqual({ name: 'song-end', beat: 32 })
    expect(nextLandmark(m, 32, 1)).toBeNull()
    // the end flag following the last clip: one rung
    expect(nextLandmark(marks({ loopEnd: 32 }), 4, 1)).toEqual({ name: 'loop-end', beat: 32 })
    expect(nextLandmark(marks({ loopEnd: 32 }), 32, 1)).toBeNull()
  })

  it('counts a hair away as being there', () => {
    expect(nextLandmark(marks({ loopStart: 8 }), 8 + 1e-9, -1)).toEqual({ name: 'song-start', beat: 0 })
  })
})

describe('stepFrom', () => {
  it('goes to the next line either way', () => {
    expect(stepFrom(4, 1, -1)).toBe(3)
    expect(stepFrom(4, 1, 1)).toBe(5)
    expect(stepFrom(4, 0.25, 1)).toBe(4.25)
  })

  it('goes from between lines to the one before or after, as a text cursor', () => {
    expect(stepFrom(4.5, 1, -1)).toBe(4)
    expect(stepFrom(4.5, 1, 1)).toBe(5)
    expect(stepFrom(6, 4, -1)).toBe(4)
    expect(stepFrom(6, 4, 1)).toBe(8)
  })

  it('treats a hair off a line as on it', () => {
    expect(stepFrom(3.9999999, 1, -1)).toBe(3)
    expect(stepFrom(4.0000001, 1, 1)).toBe(5)
  })

  it('stays put with no step', () => {
    expect(stepFrom(4, 0, 1)).toBe(4)
  })
})

describe('moveTarget', () => {
  const at = (p: Partial<Place> = {}): Place => ({
    playhead: 6, length: 64, step: 1, beatsPerBar: 4, landmarks: marks({ loopStart: 8, loopEnd: 16 }), ...p,
  })
  const go = (move: Move, p: Partial<Place> = {}) => moveTarget(move, at(p))

  it('steps and moves by bars', () => {
    expect(go({ kind: 'step', dir: -1 })).toEqual({ beat: 5, landmark: null })
    expect(go({ kind: 'step', dir: 1 }, { step: 0.5 })).toEqual({ beat: 6.5, landmark: null })
    expect(go({ kind: 'bar', dir: -1 })).toEqual({ beat: 4, landmark: null })
    expect(go({ kind: 'bar', dir: 1 })).toEqual({ beat: 8, landmark: null })
  })

  it('goes to landmarks, naming them', () => {
    expect(go({ kind: 'landmark', dir: 1 })).toEqual({ beat: 16, landmark: 'loop-end' })
    expect(go({ kind: 'landmark', dir: -1 })).toEqual({ beat: 0, landmark: 'song-start' })
    expect(go({ kind: 'home' })).toEqual({ beat: 0, landmark: 'song-start' })
    expect(go({ kind: 'end' })).toEqual({ beat: 32, landmark: 'song-end' })
  })

  it("goes nowhere from the playlist's ends, or with nowhere to go", () => {
    expect(go({ kind: 'step', dir: -1 }, { playhead: 0 })).toBeNull()
    expect(go({ kind: 'step', dir: 1 }, { playhead: 64 })).toBeNull()
    expect(go({ kind: 'home' }, { playhead: 0 })).toBeNull()
    expect(go({ kind: 'landmark', dir: 1 }, { playhead: 40 })).toBeNull()
  })

  it("keeps within the playlist's length", () => {
    expect(go({ kind: 'bar', dir: 1 }, { playhead: 63, length: 63.5 })).toEqual({ beat: 63.5, landmark: null })
  })
})

describe('trackStepForKey', () => {
  it('is ↑ or ↓ with nothing but Shift held', () => {
    expect(trackStepForKey(key('ArrowUp'))).toBe(-1)
    expect(trackStepForKey(key('ArrowDown'))).toBe(1)
    expect(trackStepForKey(key('ArrowLeft'))).toBeNull()
    // (⇧ selects); ⌘, ⌃ and ⌥ aren't track moves
    expect(trackStepForKey(key('ArrowUp', { shiftKey: true }))).toBe(-1)
    expect(trackStepForKey(key('ArrowUp', { metaKey: true }))).toBeNull()
    expect(trackStepForKey(key('ArrowDown', { altKey: true }))).toBeNull()
  })
})

describe('the info bar note', () => {
  it('says where a beat is, in bars and beats counted from 1', () => {
    expect(positionLabel(0, 4)).toBe('bar 1')
    expect(positionLabel(32, 4)).toBe('bar 9')
    expect(positionLabel(34, 4)).toBe('bar 9, beat 3')
    expect(positionLabel(33.5, 4)).toBe('bar 9, beat 2.5')
    expect(positionLabel(6, 3)).toBe('bar 3')
    // a hair off a bar (float rounding) is on it
    expect(positionLabel(31.9999999, 4)).toBe('bar 9')
  })

  it('names the landmark landed on', () => {
    expect(landmarkNote('loop-start', 32, 4)).toBe('Loop start · bar 9')
    expect(landmarkNote('last-play-start', 33.5, 4)).toBe('Last play start · bar 9, beat 2.5')
    expect(landmarkNote('song-end', 64, 4)).toBe('Song end · bar 17')
  })
})
