// the Piano Roll's keys, as plain functions of a row's notes, in steps: the
// time selection (a span of steps over every pitch) copied, cut, cleared,
// pasted and repeated as the playlist's span is (see clipboard/span.rs, a
// note across one of its ends cut there, and a paste replacing what's under
// it), ↑/↓ moving the selected notes, and ⌘B (Blade) cutting notes at the
// playhead
import { Note } from '../../types/sequencer'
import { canCut } from '../../editing/cut'
import {
  HIGHEST_PITCH, LOWEST_PITCH, addCopies, cutItems, cutNotes, deleteNotes, sortedWithPlaces,
} from './notes'

// from step `start` to step `end`
export interface StepSpan {
  start: number
  end: number
}

// what's copied from a span: its notes, each as far from the span's start
// as it was, and how long it was
export interface CopiedSpan {
  notes: Note[]
  length: number
}

// places closer than this are the same (float rounding)
const SAME = 1e-6

const every = (notes: Note[]) => notes.map((_, i) => i)

// the notes cut at both ends of the span, and which of them are inside it
export const cutAtEnds = (notes: Note[], span: StepSpan): { notes: Note[], inside: number[] } => {
  const atStart = cutNotes(notes, every(notes), span.start).notes
  const cut = cutNotes(atStart, every(atStart), span.end).notes
  const inside = cut.flatMap((note, i) =>
    (note.start + SAME >= span.start && note.start + note.length <= span.end + SAME ? [i] : []))
  return { notes: cut, inside }
}

// the notes with what's in the span taken out, a note across one of its
// ends cut there. the notes after it stay where they are
export const clearSpan = (notes: Note[], span: StepSpan): Note[] => {
  const { notes: cut, inside } = cutAtEnds(notes, span)
  return inside.length ? deleteNotes(cut, inside) : notes
}

export const copySpan = (notes: Note[], span: StepSpan): CopiedSpan => {
  const { notes: cut, inside } = cutAtEnds(notes, span)
  return {
    // (in order, as the notes are kept)
    notes: sortedWithPlaces(inside.map(i => ({ ...cut[i], start: cut[i].start - span.start }))).notes,
    length: span.end - span.start,
  }
}

// a copied span laid with its start at `at`, replacing what's under it
// (every pitch, for its whole length, even where it had no notes). kept
// before `limit`, where time ends (see timelineEnd). `span` is where it went
export const pasteSpan = (notes: Note[], copied: CopiedSpan, at: number, limit: number): {
  notes: Note[]
  span: StepSpan
} | null => {
  if(at < 0 || at >= limit) return null
  const span = { start: at, end: Math.min(at + copied.length, limit) }
  return { notes: addCopies(clearSpan(notes, span), copied.notes, at, limit).notes, span }
}

// a copy of what's in the span straight after it, replacing what's there
export const repeatSpan = (notes: Note[], span: StepSpan, limit: number) =>
  pasteSpan(notes, copySpan(notes, span), span.end, limit)

// the notes at `indices` moved `by` semitones (↑/↓: 1, ⇧↑/⇧↓: 12), as far
// as they can all go and stay on the piano, so a chord keeps its shape.
// null when they can't move that way at all
export const transposeNotes = (notes: Note[], indices: Iterable<number>, by: number): Note[] | null => {
  const chosen = new Set([...indices].filter(i => notes[i]))
  if(!chosen.size || !by) return null
  const pitches = [...chosen].map(i => notes[i].pitch)
  const allowed = by > 0
    ? Math.min(by, HIGHEST_PITCH - Math.max(...pitches))
    : Math.max(by, LOWEST_PITCH - Math.min(...pitches))
  if(!allowed) return null
  return notes.map((note, i) => (chosen.has(i) ? { ...note, pitch: note.pitch + allowed } : note))
}

// ⌘B: the notes under `step` cut there, in two. the selected ones, if any
// notes are selected, otherwise every one there. neither half is shorter
// than `shortest` (see shortestNote). null when there's nothing to cut
export const bladeNotes = (
  notes: Note[], selected: ReadonlySet<number>, step: number, shortest: number,
): Note[] | null => {
  const items = cutItems(notes, shortest)
  const from = selected.size ? [...selected] : every(notes)
  const ids = from.filter(i => items[i] && canCut(items[i], step))
  return ids.length ? cutNotes(notes, ids, step).notes : null
}
