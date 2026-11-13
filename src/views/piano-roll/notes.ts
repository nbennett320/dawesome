// the Piano Roll's editing, as plain functions of a pattern's notes: what a
// press, a drag and a key do to them. the rules shared with the playlist
// (the selection box, moving, trimming, cutting, snapping, pasting) are in
// src/editing/; these put notes through them. the component only turns the
// mouse into steps and pitches, and draws what these return
import { pasteShift, repeatShift } from '../../editing/clipboard'
import { CutItem, cutPoint, cutTargets } from '../../editing/cut'
import { Edge, TrimItem, trimAll } from '../../editing/edges'
import { Rect } from '../../editing/lasso'
import { groupLanding } from '../../editing/move'
import { SnapNote, Snapping, snapDown, snapNoteBeats, snapPoint } from '../../editing/snap'
import { Tool } from '../../editing/tools'
import { MAX_PATTERN_BARS, Note, STEPS_PER_BEAT, noteOrder } from '../../types/sequencer'
import { clamp } from '../../util/helpers'

// MIDI note numbers
export const LOWEST_PITCH = 0
export const HIGHEST_PITCH = 127
export const PITCHES = HIGHEST_PITCH - LOWEST_PITCH + 1

// how loud a new note is. there's no editing it yet
export const NEW_VELOCITY = 0.8

// a plain click makes a note this long, until a note is drawn longer or
// shorter by dragging, or trimmed: a quarter note
export const DEFAULT_LENGTH = 4

// the note the grid starts on when the pattern has none: middle C, with C3
// to C5 in view
export const MIDDLE_C = 60

const NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

// a pitch's name, with middle C (60) as C4
export const pitchName = (pitch: number): string =>
  `${NAMES[((pitch % 12) + 12) % 12]}${Math.floor(pitch / 12) - 1}`

const ACCIDENTALS: Record<string, number> = { '': 0, '#': 1, b: -1 }

// a pitch typed as pitchName gives it ("C4", "F#3", or "Gb3"), or as a
// MIDI note number, or null when it's neither or isn't a note MIDI has
export const parsePitch = (text: string): number | null => {
  const typed = text.trim()
  const named = /^([A-Ga-g])(#|b|)(-?\d+)$/.exec(typed)
  let pitch = /^\d+$/.test(typed) ? Number(typed) : null
  if(named) pitch = NAMES.indexOf(named[1].toUpperCase()) + ACCIDENTALS[named[2]] + (Number(named[3]) + 1) * 12
  return pitch !== null && pitch >= 0 && pitch <= 127 ? pitch : null
}

// the black keys of a piano: C#, D#, F#, G#, A#
export const isBlackKey = (pitch: number): boolean => [1, 3, 6, 8, 10].includes(((pitch % 12) + 12) % 12)


export const clampPitch = (pitch: number): number => clamp(Math.round(pitch), LOWEST_PITCH, HIGHEST_PITCH)

// where the Piano Roll's time runs to, in steps: as long as a pattern can
// grow (MAX_PATTERN_BARS), or the pattern's own length when that's longer
// (set under another time signature). notes are drawn, moved, trimmed and
// pasted anywhere before it, and one past the pattern's end grows it (see
// withNotes)
export const timelineEnd = (length: number, stepsPerBar: number): number =>
  Math.max(length, MAX_PATTERN_BARS * stepsPerBar)

// a note kept inside the timeline, which ends at `limit` (see timelineEnd):
// starting at or after its start, ending at or before its end, at least a
// step long (or as long as the timeline, if that's shorter), and on a key of
// the piano
export const clampNote = (note: Note, limit: number): Note => {
  const size = clamp(note.length, Math.min(1, limit), Math.max(1, limit))
  return {
    ...note,
    pitch: clampPitch(note.pitch),
    start: clamp(note.start, 0, Math.max(0, limit - size)),
    length: size,
  }
}

// ---- snapping ----

// the Piano Roll's snapping: a line every snap length, in steps (a beat is
// STEPS_PER_BEAT of them), and no playhead to snap to. `bypass` while
// Option is held
export const rollSnapping = (enabled: boolean, note: SnapNote, bypass: boolean): Snapping => ({
  enabled,
  every: snapNoteBeats(note) * STEPS_PER_BEAT,
  playhead: null,
  bypass,
})

// the shortest a note can be, in steps: the engine's MIN_NOTE
// (engine/src/daw/daw_core/sequencer.rs)
export const MIN_NOTE_LENGTH = 1 / 32

// whether anything snaps just now: snap is on, and Option isn't held
const snaps = (snap: Snapping): boolean => snap.enabled && !snap.bypass && snap.every > 0

// the shortest a note drawn, trimmed or cut can be: a step while snapping,
// and as short as a note can be while not
export const shortestNote = (snap: Snapping): number => (snaps(snap) ? 1 : MIN_NOTE_LENGTH)

// where an edge or a moved note lands, with the mouse at `step`: the nearest
// snap line, or just where the mouse is while nothing snaps
export const snapStep = (step: number, snap: Snapping): number => snapPoint(step, snap)

// where something new starts, with the mouse at `step`: the snap line at or
// before it, or the mouse's place while nothing snaps
export const snapStepDown = (step: number, snap: Snapping): number => snapDown(step, snap)

// where Play starts from, with the mouse at `step` on the ruler: the nearest
// snap line (or where the mouse is), on one inside the pattern
export const markerStep = (step: number, patternLength: number, snap: Snapping): number =>
  Math.max(0, Math.min(snapStep(step, snap), snapStepDown(patternLength - 1, snap)))

// ---- the mouse ----

// a note's box in the grid's own coordinates, which don't change as it's
// scrolled or zoomed: steps across, and rows down from the highest pitch
export const noteBox = (note: Note): Rect => ({
  left: note.start,
  top: HIGHEST_PITCH - note.pitch,
  width: note.length,
  height: 1,
})

// every note's box, with its index as its id: what a selection box
// touches, and what's under the mouse
export const noteBoxes = (notes: Note[]): Array<{ id: number, box: Rect }> =>
  notes.map((note, id) => ({ id, box: noteBox(note) }))

// what a press of the left button starts:
// - shift: a shift press, a click or a selection box (with every tool)
// - cut: a click on a note with the Cut tool
// - edge: trimming the note by the edge pressed (and the rest of the
//   selection with it)
// - move: moving the note (and the rest of the selection)
// - draw: drawing a new note, with the Draw tool on empty space (past the
//   pattern's end too, which grows it)
// - empty: a click clears the selection, a drag pans
export type PressKind = 'shift' | 'cut' | 'edge' | 'move' | 'draw' | 'empty'

export const pressKind = ({ tool, shift, onNote, onEdge }: {
  tool: Tool
  shift: boolean
  onNote: boolean
  onEdge: boolean
}): PressKind => {
  if(shift) return 'shift'
  if(tool === 'cut') return onNote ? 'cut' : 'empty'
  if(onNote) return onEdge ? 'edge' : 'move'
  return tool === 'draw' ? 'draw' : 'empty'
}

// ---- drawing ----

// a note drawn by pressing at `step` on `pitch`'s row: it starts on the snap
// line at or before the press (or right there, while nothing snaps),
// `length` long, cut short if it would run past the timeline's end
// (`limit`: see timelineEnd)
export const newNote = (step: number, pitch: number, length: number, limit: number, snap: Snapping): Note => {
  const start = clamp(snapStepDown(step, snap), 0, Math.max(0, limit - shortestNote(snap)))
  return clampNote({ pitch, start, length: Math.min(length, limit - start), velocity: NEW_VELOCITY }, limit)
}

// as a note is drawn, dragging right sets its length: to the snap line
// nearest the mouse, at least one snap length, and no further than the
// timeline's end (`limit`). while nothing snaps, to the mouse itself
export const drawnLength = (start: number, step: number, limit: number, snap: Snapping): number => {
  const shortest = shortestNote(snap)
  const length = Math.max(snapPoint(step, snap) - start, snaps(snap) ? snap.every : shortest)
  return clamp(length, shortest, Math.max(shortest, limit - start))
}

// ---- moving and trimming ----

// the notes `indices` moved together, the grabbed one (`anchor`) to start at
// `start` (already snapped) on `pitch`, the rest keeping their places about
// it. the move is held back so that none of them leaves the timeline (which
// ends at `limit`) or the piano (groupLanding, with pitches for lanes)
export const moveNotes = (
  notes: Note[],
  indices: ReadonlyArray<number>,
  anchor: number,
  start: number,
  pitch: number,
  limit: number,
): Note[] => {
  const placed = indices.filter(i => notes[i]).map(i => ({
    id: i, beat: notes[i].start, track: notes[i].pitch, length: notes[i].length,
  }))
  const landed = new Map(groupLanding(placed, anchor, start, pitch, limit, HIGHEST_PITCH).map(p => [p.id, p]))

  let changed = false
  const next = notes.map((note, i) => {
    const p = landed.get(i)
    if(!p || (p.beat === note.start && p.track === note.pitch)) return note
    changed = true
    return { ...note, start: p.beat, pitch: p.track }
  })
  return changed ? next : notes
}

// the notes `indices` as they're carried by the mouse, before they land:
// moved `steps` along and `pitches` up, both fractional (nothing snaps), and
// held back where moveNotes holds them back, so they never come away from
// where they'll land
export const carriedNotes = (
  notes: Note[],
  indices: ReadonlyArray<number>,
  steps: number,
  pitches: number,
  limit: number,
): Note[] => {
  const placed = indices.filter(i => notes[i]).map(i => ({
    id: i, beat: notes[i].start, track: notes[i].pitch, length: notes[i].length,
  }))
  if(!placed.length) return []
  const [anchor] = placed
  const landed = groupLanding(placed, anchor.id, anchor.beat + steps, anchor.track + pitches, limit, HIGHEST_PITCH)
  return landed.map(p => ({ ...notes[p.id], start: p.beat, pitch: p.track }))
}

// notes, as far as trimming goes: `shortest` long at least (see
// shortestNote), and reaching as far as the timeline does (`limit`), or the
// note, if it already reaches further
export const trimItems = (
  notes: Note[], indices: ReadonlyArray<number>, limit: number, shortest: number,
): TrimItem[] =>
  indices.filter(i => notes[i]).map(i => {
    const end = notes[i].start + notes[i].length
    return { id: i, start: notes[i].start, end, sourceStart: 0, sourceEnd: Math.max(limit, end), minLength: shortest }
  })

// the notes `indices` trimmed by one edge: the grabbed note's edge goes to
// `target` (already snapped), and the same edge of each of the others as
// far, each stopping at its own limits
export const trimNotes = (
  notes: Note[],
  indices: ReadonlyArray<number>,
  grabbed: number,
  edge: Edge,
  target: number,
  limit: number,
  shortest = 1,
): Note[] => {
  const spans = new Map(trimAll(trimItems(notes, indices, limit, shortest), grabbed, edge, target).map(s => [s.id, s]))

  let changed = false
  const next = notes.map((note, i) => {
    const span = spans.get(i)
    if(!span || (span.start === note.start && span.end === note.start + note.length)) return note
    changed = true
    return { ...note, start: span.start, length: span.end - span.start }
  })
  return changed ? next : notes
}

// ---- cutting ----

// notes, as far as cutting goes: neither half shorter than `shortest` (see
// shortestNote)
export const cutItems = (notes: Note[], shortest: number): CutItem[] =>
  notes.map((note, id) => ({ id, start: note.start, end: note.start + note.length, minLength: shortest }))

// the cut a click with the Cut tool at `step` on the note `index` makes:
// where (the snap line nearest the mouse, or the mouse's place while nothing
// snaps) and which notes (that
// one, and the rest of the selection under the cut when it's selected).
// null where it can't be cut
export const noteCut = (
  notes: Note[],
  index: number,
  selected: ReadonlySet<number>,
  step: number,
  snap: Snapping,
): { ids: number[], step: number } | null => {
  const items = cutItems(notes, shortestNote(snap))
  const item = items[index]
  if(!item) return null

  const at = cutPoint(item, step, snap)
  if(at === null) return null
  const ids = cutTargets(items, index, selected, at)
  return ids.length ? { ids, step: at } : null
}

// the notes `ids` each cut in two at `at`, both halves the same pitch and
// velocity. the first half keeps its note's place in the list and the
// second goes after every note, so no other note's index changes (until the
// list is put in order: see sortedWithPlaces). `halves` maps each note cut
// to its second half
export const cutNotes = (notes: Note[], ids: ReadonlyArray<number>, at: number): {
  notes: Note[]
  halves: Map<number, number>
} => {
  const cut = new Set(ids.filter(i => notes[i] && at > notes[i].start && at < notes[i].start + notes[i].length))
  const halves = new Map<number, number>()
  if(!cut.size) return { notes, halves }

  const seconds: Note[] = []
  const firsts = notes.map((note, i) => {
    if(!cut.has(i)) return note
    halves.set(i, notes.length + seconds.length)
    seconds.push({ ...note, start: at, length: note.start + note.length - at })
    return { ...note, length: at - note.start }
  })
  return { notes: [...firsts, ...seconds], halves }
}

// ---- copying, pasting and repeating ----

// the selected notes, in the order they're in, for the clipboard
export const copyNotes = (notes: Note[], selected: ReadonlySet<number>): Note[] =>
  [...selected].sort((a, b) => a - b).flatMap(i => (notes[i] ? [notes[i]] : []))

const spansOf = (notes: Note[]) => notes.map(note => ({ start: note.start, end: note.start + note.length }))

// copies of `copies` moved `by` steps, after the notes. a copy that would
// start at or past the timeline's end (`limit`) is left out, and one that
// crosses it is cut short. `added` is the copies' indices, to select
export const addCopies = (notes: Note[], copies: Note[], by: number, limit: number): {
  notes: Note[]
  added: number[]
} => {
  const placed = copies.flatMap(note => {
    const start = note.start + by
    if(start < 0 || start >= limit) return []
    return [{ ...note, start, length: Math.min(note.length, limit - start) }]
  })
  return {
    notes: placed.length ? [...notes, ...placed] : notes,
    added: placed.map((_, i) => notes.length + i),
  }
}

// a paste: the clipboard's notes with the earliest at `at`, the rest
// keeping their places after it, and their pitches
export const pasteNotes = (notes: Note[], clipboard: Note[], at: number, limit: number) =>
  addCopies(notes, clipboard, pasteShift(spansOf(clipboard), at), limit)

// a repeat: the selected notes copied to start where they end, so repeating
// again lays them end to end
export const repeatNotes = (notes: Note[], selected: ReadonlySet<number>, limit: number) => {
  const copies = copyNotes(notes, selected)
  return addCopies(notes, copies, repeatShift(spansOf(copies)), limit)
}

// where a paste puts the earliest note: the snap line at or before the mouse
// while it's over the grid (`mouseStep`; the mouse's place while nothing
// snaps), else the first snap line in view (the first step, while nothing
// snaps), which starts at step `firstInView`
export const pasteStep = (mouseStep: number | null, firstInView: number, snap: Snapping): number => {
  if(mouseStep !== null) return Math.max(0, snapStepDown(mouseStep, snap))
  const every = snaps(snap) ? snap.every : 1
  return Math.max(0, Math.ceil(firstInView / every - 1e-9) * every)
}

// ---- keeping notes in order ----

// notes put in order (as sortNotes puts them, and the engine keeps them),
// and where each one went: `at[i]` is the new place of `notes[i]`. what's
// kept by a note's place in the list, like the selection, follows its
// notes through `at` (see followPlaces)
export const sortedWithPlaces = (notes: Note[]): { notes: Note[], at: number[] } => {
  // (alike notes keep their order, as in a stable sort)
  const order = notes.map((_, i) => i).sort((a, b) => noteOrder(notes[a], notes[b]) || a - b)
  const at: number[] = []
  order.forEach((from, to) => { at[from] = to })
  return { notes: order.map(i => notes[i]), at }
}

// places in a list, moved to where their notes went when it was put in
// order (see sortedWithPlaces). a place past the list's end is left out
export const followPlaces = (places: Iterable<number>, at: number[]): Set<number> =>
  new Set([...places].flatMap(i => (at[i] === undefined ? [] : [at[i]])))

// ---- deleting and selecting ----

// the notes without those at `indices`
export const deleteNotes = (notes: Note[], indices: Iterable<number>): Note[] => {
  const chosen = new Set(indices)
  return notes.filter((_, i) => !chosen.has(i))
}

// every note
export const allIndices = (notes: Note[]): Set<number> => new Set(notes.map((_, i) => i))

// a selection with any index that's no longer a note left out (the notes
// changed under it: an undo, another pattern)
export const fitSelection = (selected: ReadonlySet<number>, notes: Note[]): ReadonlySet<number> =>
  ([...selected].every(i => i < notes.length) ? selected : new Set([...selected].filter(i => i < notes.length)))

// the pitch the grid is centerd on when it opens: the middle of the
// pattern's notes, or middle C
export const centerPitch = (notes: Note[]): number => {
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
