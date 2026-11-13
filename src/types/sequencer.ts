import { FolderInfo, ROOT } from './folderTree'

// the sequencers: drum patterns light steps on rows of samples, and note
// patterns hold notes for synths (instrument plugins). the rows are shared
// by every drum pattern, the synths by every note pattern. mirrors
// engine/src/daw/daw_core/sequencer.rs (see context/vst-project/04-api.md)

// a step is a 16th note
export const STEPS_PER_BEAT = 4

// the lengths a drum pattern can be, in bars
export const BAR_CHOICES = [1, 2, 4]

// and the longest a note pattern can be (it can be any number from 1)
export const MAX_NOTE_BARS = 64

// a row's volume, as a gain: 1 leaves the sample as is
export const ROW_VOLUME_MAX = 2

export interface SequencerRow {
  id: number
  path: string
  // what it's called ("Snare"), shown instead of its sample's name
  label: string | null
  volume: number
  // -1 (left) to 1 (right)
  pan: number
  muted: boolean
}

// a note in a note pattern, played on its synth. times are in steps from
// the pattern's start, and may be fractional
export interface Note {
  // MIDI note number, 0 to 127 (60 is middle C)
  pitch: number
  start: number
  length: number
  // 0 to 1
  velocity: number
}

// an instrument plugin in the Note Sequencer's list. its id is also a slot
// id, so its interface opens, docks and pops out as an effect's does. the
// engine keeps a copy for each track that plays it: the app only sees this
export interface Synth {
  id: number
  pluginId: string
  // unique in the project: "DecentSampler", then "DecentSampler 2"
  name: string
  vendor: string
  // a gain, like a row's
  volume: number
  muted: boolean
  // its plugin isn't installed: kept, with its notes and settings, silent
  missing: boolean
  hasEditor: boolean
}

// a pattern is one or the other, never both
export type PatternKind = 'drum' | 'note'

export interface Pattern {
  id: number
  // unnamed patterns are shown by their number
  name: string | null
  // left out by older projects: a drum pattern
  kind?: PatternKind
  // in steps
  length: number
  // a drum pattern's lit steps on each row, by row id
  steps: Record<string, number[]>
  // a note pattern's notes for each synth, by synth id (synths without any
  // are left out)
  notes?: Record<string, Note[]>
  // the folder it's in, in the sidebar's pattern browser (left out at the top)
  folder?: number | null
}

export const kindOf = (pattern: Pick<Pattern, 'kind'>): PatternKind => pattern.kind ?? 'drum'

const NO_NOTES: Note[] = []

// a synth's notes in a pattern. the same empty list every time, so
// selectors don't re-render for nothing
export const synthNotes = (pattern: Pick<Pattern, 'notes'>, synthId: number): Note[] =>
  pattern.notes?.[synthId] ?? NO_NOTES

// every synth's notes in a pattern, in one list
export const allNotes = (pattern: Pick<Pattern, 'notes'>): Note[] =>
  Object.values(pattern.notes ?? {}).flat()

// a pattern with a synth's notes replaced (what the engine will make of it)
export const withNotes = (pattern: Pattern, synthId: number, notes: Note[]): Pattern => {
  const next = { ...pattern.notes }
  if(notes.length) next[synthId] = notes
  else delete next[synthId]
  return { ...pattern, notes: next }
}

// a folder of patterns, in the sidebar's pattern browser. folders only sort
// patterns: they don't change how anything plays
export interface PatternFolder {
  id: number
  name: string
  // which page it's on (left out by older projects: drum)
  kind?: PatternKind
  // the folder it's in (left out at the top)
  parent?: number | null
}

// what every sequencer command returns
export interface SequencerState {
  rows: SequencerRow[]
  synths: Synth[]
  patterns: Pattern[]
  folders: PatternFolder[]
}

// what a pattern is called: its name, or its number. numbers come from ids,
// so they don't change when another pattern is deleted
export const patternLabel = (pattern: Pick<Pattern, 'id' | 'name'>): string =>
  pattern.name ?? String(pattern.id + 1)

// a sample's file name, without its folders

export const isLit = (pattern: Pattern, rowId: number, step: number): boolean =>
  !!pattern.steps[rowId]?.includes(step)

// a pattern with one step changed (what the engine will make of it)
export const withStep = (pattern: Pattern, rowId: number, step: number, on: boolean): Pattern => {
  const lit = (pattern.steps[rowId] ?? []).filter(s => s !== step)
  if(on) lit.push(step)
  lit.sort((a, b) => a - b)

  const steps = { ...pattern.steps }
  if(lit.length) steps[rowId] = lit
  else delete steps[rowId]
  return { ...pattern, steps }
}

// steps in a bar
export const stepsPerBar = (beatsPerBar: number): number => Math.max(1, beatsPerBar) * STEPS_PER_BEAT

// how many bars long a pattern is, or null when that isn't a whole number
// (the time signature changed after its length was set)
export const patternBars = (pattern: Pattern, beatsPerBar: number): number | null => {
  const bars = pattern.length / stepsPerBar(beatsPerBar)
  return Number.isInteger(bars) ? bars : null
}

// how long a pattern plays for, in seconds, at a tempo
export const patternSeconds = (pattern: Pattern, tempo: number): number =>
  tempo > 0 ? (pattern.length / STEPS_PER_BEAT) * (60 / tempo) : 0

// the step a pattern playing on its own is on, `elapsedMs` after it started
export const stepAt = (elapsedMs: number, pattern: Pattern, beatIntervalMs: number): number | null => {
  const stepMs = beatIntervalMs / STEPS_PER_BEAT
  if(stepMs <= 0 || pattern.length <= 0) return null
  return Math.floor(Math.max(0, elapsedMs) / stepMs) % pattern.length
}

// each row's lit steps in a pattern, in row order: what a pattern clip draws
export const litRows = (pattern: Pattern, rows: SequencerRow[]): number[][] =>
  rows.map(row => (pattern.steps[row.id] ?? []).filter(s => s < pattern.length))

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

  const low = Math.min(...notes.map(n => n.pitch))
  const high = Math.max(...notes.map(n => n.pitch))
  const rows = Math.max(minRows, high - low + 1)
  // the notes' range in the middle of the rows
  const top = high + Math.floor((rows - (high - low + 1)) / 2)

  return {
    rows,
    bars: notes.map(n => ({ start: n.start, end: Math.min(last, n.start + n.length), row: top - n.pitch })),
  }
}

// a pattern clip's notes, every synth's, as small bars (see noteBars)
export const clipNoteBars = (pattern: Pattern, from: number, to: number, minRows = 12): NoteBars =>
  noteBars(allNotes(pattern), pattern.length, from, to, minRows)

// what's shown faintly after a pattern's name: its length, and how many
// times it's in the playlist
export const patternHint = (pattern: Pattern, beatsPerBar: number, clips: number): string => {
  const bars = patternBars(pattern, beatsPerBar)
  const length = bars === null ? `${pattern.length} steps` : `${bars} bar${bars === 1 ? '' : 's'}`
  return clips ? `${length} · ${clips}×` : length
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
