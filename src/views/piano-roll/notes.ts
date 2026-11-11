// the Piano Roll's editing, as plain functions of a pattern's notes: what a
// press, a drag and a key do to them. the component only turns the mouse
// into steps and pitches, and draws what these return
import { Note } from '../../types/sequencer'

// MIDI note numbers
export const LOWEST_PITCH = 0
export const HIGHEST_PITCH = 127
export const PITCHES = HIGHEST_PITCH - LOWEST_PITCH + 1

// how loud a new note is. there's no editing it yet
export const NEW_VELOCITY = 0.8

// a plain click makes a note this long, until a note is drawn longer or
// shorter by dragging: a quarter note
export const DEFAULT_LENGTH = 4

// the note the grid starts on when the pattern has none: middle C, with C3
// to C5 in view
export const MIDDLE_C = 60

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

// a pitch's name, with middle C (60) as C4
export const pitchName = (pitch: number): string =>
  `${NAMES[((pitch % 12) + 12) % 12]}${Math.floor(pitch / 12) - 1}`

// the black keys of a piano: C#, D#, F#, G#, A#
export const isBlackKey = (pitch: number): boolean => [1, 3, 6, 8, 10].includes(((pitch % 12) + 12) % 12)

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value))

export const clampPitch = (pitch: number): number => clamp(Math.round(pitch), LOWEST_PITCH, HIGHEST_PITCH)

// a note kept inside a pattern `length` steps long: starting at or after
// its start, ending at or before its end, at least a step long (or as long
// as the pattern, if that's shorter), and on a key of the piano
export const clampNote = (note: Note, length: number): Note => {
  const size = clamp(note.length, Math.min(1, length), Math.max(1, length))
  return {
    ...note,
    pitch: clampPitch(note.pitch),
    start: clamp(note.start, 0, Math.max(0, length - size)),
    length: size,
  }
}

// a note drawn by pressing at `step` on `pitch`'s row: it starts on that
// whole step, cut short if it would run past the pattern's end
export const newNote = (step: number, pitch: number, length: number, patternLength: number): Note => {
  const start = clamp(Math.floor(step), 0, Math.max(0, patternLength - 1))
  return clampNote({ pitch, start, length: Math.min(length, patternLength - start), velocity: NEW_VELOCITY }, patternLength)
}

// as a note is drawn, dragging right sets its length: up to the step the
// mouse is in, and at least one step
export const drawnLength = (start: number, step: number, patternLength: number): number =>
  clamp(Math.ceil(step) - start, 1, Math.max(1, patternLength - start))

// which part of a note the mouse is on: its body moves it, its right edge
// resizes it
export type NotePart = 'body' | 'end'

export interface Hit {
  index: number
  part: NotePart
}

// the note at `step` (fractional) on `pitch`'s row: the one drawn last,
// which is on top. `edgeSteps` is how wide the right edge is, in steps; on a
// short note the edge is at most a third of it, so its body can still be
// grabbed
export const hitNote = (notes: Note[], step: number, pitch: number, edgeSteps: number): Hit | null => {
  for(let index = notes.length - 1; index >= 0; index--) {
    const note = notes[index]
    const end = note.start + note.length
    if(note.pitch === pitch && step >= note.start && step < end) {
      const edge = Math.min(edgeSteps, note.length / 3)
      return { index, part: step >= end - edge ? 'end' : 'body' }
    }
  }
  return null
}

// the notes `indices` moved by whole steps and semitones, together: the
// move is held back so that none of them leaves the pattern or the piano,
// keeping their shape
export const moveNotes = (
  notes: Note[],
  indices: number[],
  steps: number,
  semitones: number,
  patternLength: number,
): Note[] => {
  const moving = indices.map(i => notes[i]).filter(Boolean)
  if(!moving.length) return notes

  const earliest = Math.min(...moving.map(n => n.start))
  const latest = Math.max(...moving.map(n => n.start + n.length))
  const lowest = Math.min(...moving.map(n => n.pitch))
  const highest = Math.max(...moving.map(n => n.pitch))

  const dStep = clamp(Math.round(steps), -earliest, Math.max(-earliest, patternLength - latest))
  const dPitch = clamp(Math.round(semitones), LOWEST_PITCH - lowest, HIGHEST_PITCH - highest)
  if(!dStep && !dPitch) return notes

  const chosen = new Set(indices)
  return notes.map((note, i) => (chosen.has(i)
    ? { ...note, start: note.start + dStep, pitch: note.pitch + dPitch }
    : note))
}

// the notes `indices` made longer (or shorter) by whole steps, each kept at
// least a step long and inside the pattern
export const resizeNotes = (notes: Note[], indices: number[], steps: number, patternLength: number): Note[] => {
  const by = Math.round(steps)
  if(!by) return notes

  const chosen = new Set(indices)
  return notes.map((note, i) => (chosen.has(i)
    ? { ...note, length: clamp(note.length + by, Math.min(1, note.length), Math.max(1, patternLength - note.start)) }
    : note))
}

// the notes without those at `indices`
export const deleteNotes = (notes: Note[], indices: number[]): Note[] => {
  const chosen = new Set(indices)
  return notes.filter((_, i) => !chosen.has(i))
}

// the notes a rubber band from (step, pitch) to (step, pitch) touches,
// whichever way it was dragged
export const notesInBox = (
  notes: Note[],
  from: { step: number, pitch: number },
  to: { step: number, pitch: number },
): number[] => {
  const left = Math.min(from.step, to.step)
  const right = Math.max(from.step, to.step)
  const low = Math.min(from.pitch, to.pitch)
  const high = Math.max(from.pitch, to.pitch)

  return notes.flatMap((note, i) => (
    note.pitch >= low && note.pitch <= high && note.start < right && note.start + note.length > left ? [i] : []
  ))
}

// shift-clicking a note: in the selection, or out of it
export const toggleSelected = (selected: number[], index: number): number[] =>
  (selected.includes(index) ? selected.filter(i => i !== index) : [...selected, index].sort((a, b) => a - b))

// every note
export const allIndices = (notes: Note[]): number[] => notes.map((_, i) => i)

// a selection with any index that's no longer a note left out (the notes
// changed under it: an undo, another pattern)
export const fitSelection = (selected: number[], notes: Note[]): number[] =>
  (selected.every(i => i < notes.length) ? selected : selected.filter(i => i < notes.length))

// the pitch the grid is centred on when it opens: the middle of the
// pattern's notes, or middle C
export const centrePitch = (notes: Note[]): number => {
  if(!notes.length) return MIDDLE_C
  const low = Math.min(...notes.map(n => n.pitch))
  const high = Math.max(...notes.map(n => n.pitch))
  return Math.round((low + high) / 2)
}

// whether two lists of notes are the same notes, in the same order
export const sameNotes = (a: Note[], b: Note[]): boolean =>
  a === b || (a.length === b.length && a.every((n, i) => {
    const m = b[i]
    return n.pitch === m.pitch && n.start === m.start && n.length === m.length && n.velocity === m.velocity
  }))
