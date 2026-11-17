// moving the playhead with the keyboard, as a text cursor moves: ←/→ by a
// snap step, ⌥←/⌥→ by a bar, ⌘←/⌘→ to the next landmark that way, and
// Home/End to the song's start and end. kept free of p5 so it can be tested
// on its own. everything is in beats
import { commandHeld, isMac } from '../../util/shortcuts'
import { PlaylistTransport } from '../../types/transport'
import { loopMarks } from './sketch/loopRegion'

export type Direction = -1 | 1

export type Move =
  | { kind: 'step', dir: Direction }
  | { kind: 'bar', dir: Direction }
  | { kind: 'landmark', dir: Direction }
  | { kind: 'home' }
  | { kind: 'end' }

type KeyPress = Pick<KeyboardEvent, 'key' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey'>

// the move a key press asks for, if any. ⌘ is ⌃ on other systems, and ⌥ is
// Alt. with Shift held it's a selection, not a move
export const moveForKey = (ev: KeyPress, mac: boolean = isMac()): Move | null => {
  if(ev.shiftKey) return null

  const dir = ({ ArrowLeft: -1, ArrowRight: 1 } as Record<string, Direction | undefined>)[ev.key]
  if(dir) {
    if(commandHeld(ev, mac)) return { kind: 'landmark', dir }
    if(ev.metaKey || ev.ctrlKey) return null
    return { kind: ev.altKey ? 'bar' : 'step', dir }
  }

  if(ev.metaKey || ev.ctrlKey || ev.altKey) return null
  if(ev.key === 'Home') return { kind: 'home' }
  if(ev.key === 'End') return { kind: 'end' }
  return null
}

export type LandmarkName = 'last-play-start' | 'loop-start' | 'song-start' | 'loop-end' | 'song-end'

// the places ⌘←/⌘→ go, where there are any
export interface Landmarks {
  lastPlayStart: number | null
  loopStart: number | null
  loopEnd: number | null
  songEnd: number
}

type LandmarkTransport = Pick<
  PlaylistTransport,
  'loopStartBeats' | 'loopEndBeats' | 'loopRegion' | 'loopEnabled' | 'songEndMs' | 'beatIntervalMs' | 'lastPlayStartBeats'
>

// the landmarks the transport describes. the loop's are the ones the ruler
// shows: a region's edges, or with none, the end flag while looping
export const landmarks = (t: LandmarkTransport): Landmarks => {
  const { region, flag } = loopMarks(t)
  return {
    lastPlayStart: t.lastPlayStartBeats,
    loopStart: region?.start ?? null,
    loopEnd: region?.end ?? flag,
    songEnd: t.beatIntervalMs > 0 ? t.songEndMs / t.beatIntervalMs : 0,
  }
}

// the order ⌘← and ⌘→ try them in
const LADDERS: Record<Direction, LandmarkName[]> = {
  [-1]: ['last-play-start', 'loop-start', 'song-start'],
  [1]: ['loop-end', 'song-end'],
}

const beatOf = (marks: Landmarks, name: LandmarkName): number | null => ({
  'last-play-start': marks.lastPlayStart,
  'loop-start': marks.loopStart,
  'song-start': 0,
  'loop-end': marks.loopEnd,
  'song-end': marks.songEnd,
}[name])

// positions closer than this count as the same place (float rounding)
const SAME = 1e-6

// where ⌘← or ⌘→ goes from `from`: the first landmark on its ladder that's
// that way. so pressing again carries on to the next one (the last play
// start, then the loop's start, then the song's), and one that's missing,
// or where the playhead is already, is passed over
export const nextLandmark = (
  marks: Landmarks, from: number, dir: Direction,
): { name: LandmarkName, beat: number } | null => {
  for(const name of LADDERS[dir]) {
    const beat = beatOf(marks, name)
    if(beat !== null && (beat - from) * dir > SAME) return { name, beat }
  }
  return null
}

// the line `size` apart that's next from `from`, as a text cursor moves: from
// between two lines, ← goes to the one before and → to the one after
export const stepFrom = (from: number, size: number, dir: Direction): number => {
  if(!(size > 0)) return from
  const lines = from / size
  const n = dir < 0 ? Math.ceil(lines - SAME) - 1 : Math.floor(lines + SAME) + 1
  return n * size
}

export interface Place {
  // where the playhead is
  playhead: number
  // the playlist's length: nothing goes past it
  length: number
  // one ←/→ step
  step: number
  beatsPerBar: number
  landmarks: Landmarks
}

// where a move goes, and the landmark it lands on (if it's to one). null
// when it goes nowhere: already at the start, or no landmark that way
export const moveTarget = (move: Move, at: Place): { beat: number, landmark: LandmarkName | null } | null => {
  const to = (beat: number, landmark: LandmarkName | null = null) => {
    const clamped = Math.min(Math.max(beat, 0), at.length)
    return Math.abs(clamped - at.playhead) > SAME ? { beat: clamped, landmark } : null
  }

  switch(move.kind) {
    case 'step': return to(stepFrom(at.playhead, at.step, move.dir))
    case 'bar': return to(stepFrom(at.playhead, at.beatsPerBar, move.dir))
    case 'home': return to(0, 'song-start')
    case 'end': return to(at.landmarks.songEnd, 'song-end')
    case 'landmark': {
      const next = nextLandmark(at.landmarks, at.playhead, move.dir)
      return next ? to(next.beat, next.name) : null
    }
  }
}
