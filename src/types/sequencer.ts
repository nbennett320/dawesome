import { FolderInfo, ROOT } from './folderTree'
import { fileName } from '../util/helpers'

// the sequencer: patterns hold notes for rows, and a row is an instrument,
// a sample or a synth (an instrument plugin). a drum step is a 1-step note
// at its row's step pitch. mirrors engine/src/daw/daw_core/sequencer.rs (see
// context/pattern-merge-project/04-api.md)

// a step is a 16th note
export const STEPS_PER_BEAT = 4

// the longest a pattern can be, in bars
export const MAX_PATTERN_BARS = 128

// the pitch a new row's clicked steps play, and that plays a sample as it
// is: middle C
export const DEFAULT_PITCH = 60

// how loud a clicked step is
export const STEP_VELOCITY = 1

// a row's volume, as a gain: 1 leaves the instrument as it is
export const ROW_VOLUME_MAX = 2

// the rev of a row or pattern the frontend changed itself, before the
// engine has answered (see withStep). it's never one of the engine's, so
// the answer always replaces it
export const UNCONFIRMED = -1

export interface SampleInstrument {
  kind: 'sample'
  path: string
  // 0 to 127: the note that plays the sample as it is
  samplePitch: number
  // false: a note plays the whole sample (one-shot). true: it stops at the
  // note's end
  gated: boolean
}

// an instrument plugin. its slot is what its interface opens, docks and
// pops out by, as an effect's does. the engine keeps a copy for each track
// that plays it: the app only sees this
export interface SynthInstrument {
  kind: 'synth'
  slot: number
  pluginId: string
  // unique among the synth rows: "DecentSampler", then "DecentSampler 2"
  name: string
  vendor: string
  // its plugin isn't installed: kept, with its notes and settings, silent
  missing: boolean
  hasEditor: boolean
}

export type Instrument = SampleInstrument | SynthInstrument

// a row of the sequencer: one instrument, and how a pattern's steps play on
// it. its id is what patterns' notes and every row command go by (a synth
// row's id is not its slot)
export interface Row<I extends Instrument = Instrument> {
  id: number
  // new whenever the engine changes the row (not when it moves). compared
  // only for being the same (see sameRow)
  rev: number
  // what it's called ("Snare"), shown instead of its instrument's name
  label: string | null
  // a gain, 0 to ROW_VOLUME_MAX
  volume: number
  // -1 (left) to 1 (right)
  pan: number
  muted: boolean
  // 0 to 127: what a clicked step plays
  stepPitch: number
  instrument: I
}

export const isSampleRow = (row: Row): row is Row<SampleInstrument> => row.instrument.kind === 'sample'
export const isSynthRow = (row: Row): row is Row<SynthInstrument> => row.instrument.kind === 'synth'

// the synth row whose plugin is in a slot. plugin windows, docking and tabs
// go by slot, the sequencer by row id: never compare one with the other
export const synthRowBySlot = (rows: Row[], slot: number): Row<SynthInstrument> | undefined =>
  rows.find((row): row is Row<SynthInstrument> => isSynthRow(row) && row.instrument.slot === slot)

// a file's name without its folders or extension, as the engine's
// file_stem: "kick.wav" is "kick", ".hidden" stays
const fileStem = (path: string): string => {
  const name = fileName(path)
  const dot = name.lastIndexOf('.')
  return dot > 0 ? name.slice(0, dot) : name
}

// what a row is called: its label, its sample's name, or its synth's
export const rowName = (row: Row): string =>
  row.label ?? (row.instrument.kind === 'sample' ? fileStem(row.instrument.path) : row.instrument.name)

// a note, played on its row's instrument. times are in steps from the
// pattern's start, and may be fractional
export interface Note {
  // MIDI note number, 0 to 127 (60 is middle C)
  pitch: number
  start: number
  length: number
  // 0 to 1
  velocity: number
}

export interface Pattern {
  id: number
  // new whenever the engine changes the pattern, and UNCONFIRMED while a
  // change made here hasn't been answered. compared only for being the same
  rev: number
  // unnamed patterns are shown by their number
  name: string | null
  // in steps
  length: number
  // in steps: auto-trim never shortens the pattern below it
  floor: number
  // each row's notes, by row id, in order (see sortNotes). rows without
  // notes are left out
  notes: Record<string, Note[]>
  // the folder it's in, in the sidebar's pattern browser (left out at the top)
  folder?: number | null
}

// a row's notes are kept in this order, by the engine as well: by start,
// then pitch. notes alike in both keep their order
export const noteOrder = (a: Note, b: Note): number => a.start - b.start || a.pitch - b.pitch

// notes in order (a stable sort, so notes alike keep theirs)
export const sortNotes = (notes: Note[]): Note[] => [...notes].sort(noteOrder)

const NO_NOTES: Note[] = []

// a row's notes in a pattern. the same empty list every time, so
// selectors don't re-render for nothing
export const rowNotes = (pattern: Pick<Pattern, 'notes'>, rowId: number): Note[] =>
  pattern.notes[rowId] ?? NO_NOTES

// the length a pattern grows to so a note ending at `end` fits: to the end
// of the bar holding that end. never shorter than it was (even past the
// cap), never grown past MAX_PATTERN_BARS. the engine's grown_length is the
// same
export const grownLength = (length: number, end: number, stepsPerBar: number): number => {
  const cap = MAX_PATTERN_BARS * stepsPerBar
  const fit = Math.max(1, Math.ceil((end - 1e-9) / stepsPerBar)) * stepsPerBar
  return Math.max(length, Math.min(fit, cap))
}

const sameNote = (a: Note, b: Note): boolean =>
  a.pitch === b.pitch && a.start === b.start && a.length === b.length && a.velocity === b.velocity

const noteKey = (note: Note): string => `${note.pitch}:${note.start}:${note.length}:${note.velocity}`

// where the notes in `next` that aren't in `prev` end, the furthest of
// them (0 when none was added). notes are compared on all they are, and a
// note in `next` twice but in `prev` once counts once as added. the
// engine's added_end is the same
export const addedEnd = (prev: Note[], next: Note[]): number => {
  // (the notes both lists start and end with alike aren't looked at: an
  // edit usually changes a few notes)
  let head = 0
  while(head < prev.length && head < next.length && sameNote(prev[head], next[head])) head += 1
  let tail = 0
  while(tail < prev.length - head && tail < next.length - head
    && sameNote(prev[prev.length - 1 - tail], next[next.length - 1 - tail])) tail += 1

  const had = new Map<string, number>()
  prev.slice(head, prev.length - tail).forEach(note => had.set(noteKey(note), (had.get(noteKey(note)) ?? 0) + 1))
  let end = 0
  next.slice(head, next.length - tail).forEach(note => {
    const count = had.get(noteKey(note)) ?? 0
    if(count > 0) had.set(noteKey(note), count - 1)
    else end = Math.max(end, note.start + note.length)
  })
  return end
}

// a pattern with a row's notes replaced (what the engine will make of
// it, unconfirmed until it answers), grown to fit the notes added (see
// addedEnd and grownLength). `notes` must be in order already
export const withNotes = (pattern: Pattern, rowId: number, notes: Note[], stepsPerBar: number): Pattern => {
  const end = addedEnd(rowNotes(pattern, rowId), notes)
  const next = { ...pattern.notes }
  if(notes.length) next[rowId] = notes
  else delete next[rowId]
  const length = end > 0 ? grownLength(pattern.length, end, stepsPerBar) : pattern.length
  return { ...pattern, rev: UNCONFIRMED, length, notes: next }
}

// a pattern with a row's step (a whole step) lit or cleared, as the engine
// does it: lit, a 1-step note at the row's step pitch, unless one's there,
// and the pattern grown to the end of the step's bar (a step past its end
// was clicked), whether the note was there or not. cleared, every note of
// the row at that step and pitch goes
export const withStep = (pattern: Pattern, row: Row, step: number, on: boolean, stepsPerBar: number): Pattern => {
  const notes = rowNotes(pattern, row.id)
  const atStep = (note: Note) => note.start === step && note.pitch === row.stepPitch
  const length = on ? grownLength(pattern.length, step + 1, stepsPerBar) : pattern.length
  if(on === notes.some(atStep)) return length === pattern.length ? pattern : { ...pattern, rev: UNCONFIRMED, length }

  return withNotes({ ...pattern, length }, row.id, on
    ? sortNotes([...notes, { pitch: row.stepPitch, start: step, length: 1, velocity: STEP_VELOCITY }])
    : notes.filter(note => !atStep(note)), stepsPerBar)
}

// where a pattern's last note ends, of any row (notes past its end too), in
// steps. 0 when it has none. (the notes are sorted by start, not end, so
// each one is looked at)
export const lastEnd = (pattern: Pick<Pattern, 'notes'>): number => {
  let end = 0
  for(const notes of Object.values(pattern.notes)) {
    for(const note of notes) end = Math.max(end, note.start + note.length)
  }
  return end
}

// the length a pattern trims to (auto-trim, after a clear): to the end of
// the bar holding its last note's end (see lastEnd), never below its floor
// or a bar, never longer than it is. the engine's trimmed_length is the same
export const trimmedLength = (pattern: Pattern, stepsPerBar: number): number => {
  const end = lastEnd(pattern)
  const fit = end > 0 ? Math.ceil((end - 1e-9) / stepsPerBar) * stepsPerBar : 0
  return Math.min(pattern.length, Math.max(pattern.floor, fit, stepsPerBar))
}

// a pattern trimmed (see trimmedLength), as the engine will trim it
// (unconfirmed until it answers). the pattern itself when there's nothing
// to trim
export const withTrim = (pattern: Pattern, stepsPerBar: number): Pattern => {
  const length = trimmedLength(pattern, stepsPerBar)
  return length === pattern.length ? pattern : { ...pattern, rev: UNCONFIRMED, length }
}

// a row's notes in a pattern `length` steps long, as the steps they light,
// in order. null when they aren't plain steps, and the row shows them as
// notes instead. only notes starting inside the pattern count: each must
// start on a whole step, at the row's step pitch, and be a step long (on a
// one-shot sample row, any length). velocity doesn't matter, and two notes
// on one step light it once. a view never changes the notes
export const stepView = (row: Row, notes: Note[], length: number): number[] | null => {
  const anyLength = row.instrument.kind === 'sample' && !row.instrument.gated
  const lit: number[] = []
  for(const note of notes) {
    if(note.start >= length) continue
    const plain = Number.isInteger(note.start) && note.pitch === row.stepPitch
      && (anyLength || note.length === 1)
    if(!plain) return null
    if(lit[lit.length - 1] !== note.start) lit.push(note.start)
  }
  return lit
}

// how many steps after a pattern's end are shown greyed, for a click to
// grow it (see withStep): a bar, or what's left of the longest it can be
export const greyedSteps = (length: number, stepsPerBar: number): number =>
  Math.max(0, Math.min(stepsPerBar, MAX_PATTERN_BARS * stepsPerBar - length))

// a folder of patterns, in the sidebar's pattern browser. folders only sort
// patterns: they don't change how anything plays
export interface PatternFolder {
  id: number
  name: string
  // the folder it's in (left out at the top)
  parent?: number | null
}

// what most sequencer commands return
export interface SequencerState {
  rows: Row[]
  patterns: Pattern[]
  folders: PatternFolder[]
}

// what a command that changed one pattern returns (a step, notes, its bars)
export interface PatternChange {
  pattern: Pattern
}

// what a command that changed one row's own settings returns (its volume,
// pan, mute, label or sample)
export interface RowChange {
  row: Row
}

// the object `had` holds for every item the engine hasn't changed (`same`),
// and `had` itself when nothing in the list changed, so selectors and
// memoised rows see the same references and don't re-render
export const keepUnchanged = <T extends { id: number }>(
  had: readonly T[], got: readonly T[], same: (had: T, got: T) => boolean,
): T[] => {
  const byId = new Map(had.map(item => [item.id, item]))
  const kept = got.map(item => {
    const old = byId.get(item.id)
    return old && same(old, item) ? old : item
  })
  const changed = kept.length !== had.length || kept.some((item, i) => item !== had[i])
  return changed ? kept : had as T[]
}

export const samePattern = (a: Pattern, b: Pattern): boolean => a.rev === b.rev

// (a synth's missing and hasEditor are its plugin's, which the rev doesn't
// cover)
export const sameRow = (a: Row, b: Row): boolean => a.rev === b.rev
  && (a.instrument.kind !== 'synth' || (b.instrument.kind === 'synth'
    && a.instrument.missing === b.instrument.missing && a.instrument.hasEditor === b.instrument.hasEditor))

// a pattern the engine sent, holding `had`'s list of notes for each row
// whose notes are the same, so the rows showing them don't re-render. (an
// edit changes one row's notes: the rest arrive as new copies)
export const keepNotes = (had: Pattern, got: Pattern): Pattern => {
  const notes: Record<string, Note[]> = {}
  Object.entries(got.notes).forEach(([rowId, list]) => {
    const old = had.notes[rowId]
    const same = old !== undefined && old.length === list.length && old.every((note, i) => sameNote(note, list[i]))
    notes[rowId] = same ? old : list
  })
  return { ...got, notes }
}

// a row just added, and the sequencer with it
export interface AddedRow {
  rowId: number
  sequencer: SequencerState
}

// what an unnamed pattern is called: "Sequence" and its number. numbers
// come from ids, so they don't change when another pattern is deleted.
// (new patterns are given a name like this by the engine; unnamed ones were
// renamed blank)
export const unnamedLabel = (pattern: Pick<Pattern, 'id'>): string => `Sequence ${pattern.id + 1}`

// what a pattern is called: its name, or as an unnamed one
export const patternLabel = (pattern: Pick<Pattern, 'id' | 'name'>): string =>
  pattern.name ?? unnamedLabel(pattern)

// steps in a bar
export const stepsPerBar = (beatsPerBar: number): number => Math.max(1, beatsPerBar) * STEPS_PER_BEAT

// how many bars long a pattern is, or null when that isn't a whole number
// (the time signature changed after its length was set)
export const patternBars = (pattern: Pattern, beatsPerBar: number): number | null => {
  const bars = pattern.length / stepsPerBar(beatsPerBar)
  return Number.isInteger(bars) ? bars : null
}

// the bars a pattern reaches into (its last one may not be whole), as the
// bars counter shows them
export const barsReached = (pattern: Pattern, beatsPerBar: number): number =>
  Math.ceil(pattern.length / stepsPerBar(beatsPerBar))

// how long a pattern plays for, in seconds, at a tempo
export const patternSeconds = (pattern: Pattern, tempo: number): number =>
  tempo > 0 ? (pattern.length / STEPS_PER_BEAT) * (60 / tempo) : 0

// the step a pattern playing on its own is on, `elapsedMs` after it started
export const stepAt = (elapsedMs: number, pattern: Pick<Pattern, 'length'>, beatIntervalMs: number): number | null => {
  const stepMs = beatIntervalMs / STEPS_PER_BEAT
  if(stepMs <= 0 || pattern.length <= 0) return null
  return Math.floor(Math.max(0, elapsedMs) / stepMs) % pattern.length
}

// the step of its pattern, `length` steps long, that a clip starting
// `start` steps into it plays `t` steps after the clip's start: the clip
// plays the pattern end to end, over and over (pattern_step_at in
// engine/src/daw/daw_core/audiograph.rs)
export const patternStepAt = (start: number, t: number, length: number): number => {
  const at = (start + t) % length
  return at < 0 ? at + length : at
}

// one time through its pattern that a clip plays: the part of the pattern
// it plays then (`from` up to `to`, steps of the pattern), and where that
// time through starts, in steps from where the clip would have the
// pattern's first step (a whole number of patterns). Pass in
// engine/src/daw/daw_core/audiograph.rs
export interface Pass {
  from: number
  to: number
  at: number
}

// every time through its pattern, `patternLength` steps long, that a clip
// plays, in order: from `start` steps into the pattern (not wrapped), for
// `length` steps. PatternNode::passes in engine/src/daw/daw_core/audiograph.rs
export const clipPasses = (start: number, length: number, patternLength: number): Pass[] => {
  if(!(patternLength > 0)) return []
  const end = start + length
  const passes: Pass[] = []
  for(let at = start - patternStepAt(start, 0, patternLength); at < end; at += patternLength) {
    passes.push({ from: Math.max(start - at, 0), to: Math.min(end - at, patternLength), at })
  }
  return passes
}

// where a clip starts its pattern over, in steps from the clip's start (for
// the faint repeat lines): k·L − s for every whole k with s < k·L < s + c
export const repeatLines = (start: number, length: number, patternLength: number): number[] =>
  clipPasses(start, length, patternLength).slice(1).map(pass => pass.at - start)

export interface NoteBars {
  rows: number
  bars: Array<{ start: number, end: number, row: number }>
}

// notes as small bars: those that start in the part of a pattern played
// (`from` to `to`, in steps), as it plays them, each cut where that part or
// the pattern (`length` steps) ends. `row` is 0 for the highest pitch shown,
// and `rows` how many there are: the notes' range, with a little room so a
// single pitch isn't drawn as one fat bar
export const noteBars = (
  all: Note[],
  length: number,
  from: number,
  to: number,
  minRows = 12,
): NoteBars => {
  const last = Math.min(to, length)
  const notes = all.filter(n => n.start >= from && n.start < last)
  if(!notes.length) return { rows: 0, bars: [] }

  // (a loop: a spread of tens of thousands of pitches is too many arguments)
  let low = notes[0].pitch
  let high = notes[0].pitch
  notes.forEach(n => {
    low = Math.min(low, n.pitch)
    high = Math.max(high, n.pitch)
  })
  const rows = Math.max(minRows, high - low + 1)
  // the notes' range in the middle of the rows
  const top = high + Math.floor((rows - (high - low + 1)) / 2)

  return {
    rows,
    bars: notes.map(n => ({ start: n.start, end: Math.min(last, n.start + n.length), row: top - n.pitch })),
  }
}

// what's shown faintly after a pattern's name: its length, and how many
// times it's in the playlist
export const patternHint = (pattern: Pattern, beatsPerBar: number, clips: number): string => {
  const bars = patternBars(pattern, beatsPerBar)
  const length = bars === null ? `${pattern.length} steps` : `${bars} bar${bars === 1 ? '' : 's'}`
  return clips ? `${length} · ${clips}×` : length
}

// each row's kind of instrument, by row id
export type RowKinds = ReadonlyMap<number, Instrument['kind']>

// what a pattern holds notes for: sample rows, synth rows, or both (the
// pattern browser's badges). notes past its end count: it holds them still
export interface Holds {
  samples: boolean
  synths: boolean
}

export const patternHolds = (pattern: Pattern, kinds: RowKinds): Holds => {
  let samples = false
  let synths = false
  Object.entries(pattern.notes).forEach(([rowId, notes]) => {
    if(!notes.length) return
    const kind = kinds.get(Number(rowId))
    samples ||= kind === 'sample'
    synths ||= kind === 'synth'
  })
  return { samples, synths }
}

// the patterns whose label has the search in it, keeping their order
export const searchPatterns = (patterns: Pattern[], query: string): Pattern[] => {
  const search = query.trim().toLowerCase()
  if(!search) return patterns
  return patterns.filter(p => patternLabel(p).toLowerCase().includes(search))
}

// pattern folders in the browser's tree are keyed by their ids, as strings
// (the top is ROOT)
export const folderKey = (id: number | null | undefined): string =>
  (id === null || id === undefined ? ROOT : String(id))

export const folderIdOf = (key: string): number | null => (key === ROOT ? null : Number(key))

export const patternFolderInfos = (folders: PatternFolder[]): FolderInfo[] =>
  folders.map(f => ({ key: folderKey(f.id), name: f.name, parent: folderKey(f.parent) }))

// where a folder is, by name from the top: "Drums / Fills"
export const folderPathLabel = (folders: PatternFolder[], id: number | null | undefined): string => {
  const byId = new Map(folders.map(f => [f.id, f]))
  const names: string[] = []
  const seen = new Set<number>()
  let folder = id === null || id === undefined ? undefined : byId.get(id)
  while(folder && !seen.has(folder.id)) {
    seen.add(folder.id)
    names.unshift(folder.name)
    folder = folder.parent === null || folder.parent === undefined ? undefined : byId.get(folder.parent)
  }
  return names.join(' / ')
}
