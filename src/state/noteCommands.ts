// the Piano Roll's commands, run the same way wherever they came from: a
// key, the Edit menu, or the system's copy, cut and paste events. like the
// clips' (see clipCommands.ts), each does nothing unless its view is the
// focused one
import { Note } from '../types/sequencer'
import {
  allIndices, copyNotes, deleteNotes, followPlaces, pasteNotes, repeatNotes, sortedWithPlaces,
} from '../views/piano-roll/notes'
import {
  StepSpan, bladeNotes, clearSpan, copySpan, pasteSpan, repeatSpan, transposeNotes,
} from '../views/piano-roll/span'

// the commands with a ⌘ shortcut, which can arrive by more than one route
// (the key, the menu, the system's clipboard events; see sameCommand), and
// those only a key gives
export type NoteShortcut = 'copy' | 'cut' | 'paste' | 'repeat' | 'select-all' | 'blade'
// ↑/↓ move the selected notes a semitone, and with ⇧ an octave
export type NoteCommand = NoteShortcut | 'delete' | 'escape' | 'up' | 'down' | 'octave-up' | 'octave-down'

// the notes copied: the page's own clipboard (not the system's, nor the
// clips'), kept as another pattern or synth is shown. copied from a time
// selection, they're a span: as far from its start as they were, and
// `spanLength` long (null for notes copied as notes)
let clipboard: Note[] = []
let spanLength: number | null = null

export const noteClipboard = (): Note[] => clipboard

// for tests
export const clearNoteClipboard = (): void => {
  clipboard = []
  spanLength = null
}

const TRANSPOSE: Partial<Record<NoteCommand, number>> = { up: 1, down: -1, 'octave-up': 12, 'octave-down': -12 }

export interface NoteTarget {
  // the row's notes in the pattern shown, in order, and which are selected
  notes: Note[]
  selected: ReadonlySet<number>
  // where the Piano Roll's time runs to, in steps (see timelineEnd): no
  // copy starts at or past it
  limit: number
  // the step a paste puts the earliest note at (see pasteStep)
  pasteAt: number
  // whether the Piano Roll is the focused view
  focused: boolean
  // the time selection, if it covers any time: copy, cut, repeat and delete
  // act on it rather than on the selected notes
  span: StepSpan | null
  // where the playhead is, in steps: ⌘B cuts there, and a copied span is
  // pasted there
  playhead: number
  // the shortest a note ⌘B leaves can be (see shortestNote)
  shortest: number
}

// whether a command takes notes away, which trims the bars it leaves empty
// at the pattern's end (see trimmedLength)
export const clearsNotes = (command: NoteCommand): boolean => command === 'delete' || command === 'cut'

// what a command changed: the notes (in order), what's selected after, and
// the time selection after (null: none; left out: as it was)
export interface NoteOutcome {
  notes?: Note[]
  selected?: ReadonlySet<number>
  span?: StepSpan | null
}

// a command's notes put in order, the selection following them
const inOrder = (outcome: NoteOutcome, selected: ReadonlySet<number>): NoteOutcome => {
  if(!outcome.notes) return outcome
  const sorted = sortedWithPlaces(outcome.notes)
  return { ...outcome, notes: sorted.notes, selected: followPlaces(outcome.selected ?? selected, sorted.at) }
}

// what a command does to the notes, before they're put in order
const changeNotes = (command: NoteCommand, target: NoteTarget): NoteOutcome | null => {
  if(!target.focused) return null
  const { notes, selected, limit, pasteAt, span, playhead } = target
  const none = new Set<number>()

  // a span copied: a paste replaces as much at the playhead
  const copy = (from: StepSpan) => {
    const copied = copySpan(notes, from)
    clipboard = copied.notes
    spanLength = copied.length
  }

  switch(command) {
    case 'copy':
      if(span) {
        copy(span)
        return {}
      }
      if(!selected.size) return null
      clipboard = copyNotes(notes, selected)
      spanLength = null
      return {}
    case 'cut':
      if(span) {
        copy(span)
        return { notes: clearSpan(notes, span), selected: none }
      }
      if(!selected.size) return null
      clipboard = copyNotes(notes, selected)
      spanLength = null
      return { notes: deleteNotes(notes, selected), selected: none }
    case 'paste': {
      if(spanLength !== null) {
        const pasted = pasteSpan(notes, { notes: clipboard, length: spanLength }, playhead, limit)
        return pasted && { notes: pasted.notes, selected: none, span: pasted.span }
      }
      if(!clipboard.length) return null
      const pasted = pasteNotes(notes, clipboard, pasteAt, limit)
      return pasted.added.length ? { notes: pasted.notes, selected: new Set(pasted.added) } : null
    }
    case 'repeat': {
      if(span) {
        const repeated = repeatSpan(notes, span, limit)
        return repeated && { notes: repeated.notes, selected: none, span: repeated.span }
      }
      if(!selected.size) return null
      const repeated = repeatNotes(notes, selected, limit)
      return repeated.added.length ? { notes: repeated.notes, selected: new Set(repeated.added) } : null
    }
    case 'select-all': return { selected: allIndices(notes), span: null }
    case 'delete':
      if(span) return { notes: clearSpan(notes, span), selected: none }
      return selected.size ? { notes: deleteNotes(notes, selected), selected: none } : null
    case 'escape': return { selected: none, span: null }
    case 'blade': {
      const cut = bladeNotes(notes, selected, playhead, target.shortest)
      return cut && { notes: cut }
    }
    case 'up':
    case 'down':
    case 'octave-up':
    case 'octave-down': {
      const moved = transposeNotes(notes, selected, TRANSPOSE[command] ?? 0)
      return moved && { notes: moved }
    }
    default: return null
  }
}

// run a command on the notes in `target`. null when it does nothing: the
// Piano Roll isn't focused, nothing's selected to copy, cut, repeat or
// delete, there's nothing to paste, or every copy would be past the
// timeline's end (and what's selected stays as it was). copy and cut fill
// the clipboard. with a time selection, copy, cut, repeat and delete act on
// its span (and paste lays a copied span at the playhead)
export const noteCommand = (command: NoteCommand, target: NoteTarget): NoteOutcome | null => {
  const outcome = changeNotes(command, target)
  return outcome && inOrder(outcome, target.selected)
}

// the Piano Roll registers how it runs a command, so the Edit menu's items
// reach it without the menu's code knowing about the grid. it does nothing
// unless the Piano Roll is the focused view (and the Playlist's own runner
// does nothing unless the Playlist is)
let runner: ((command: NoteShortcut, source: 'menu') => void) | null = null

export const setNoteCommandRunner = (run: ((command: NoteShortcut, source: 'menu') => void) | null) => {
  runner = run
}

export const runNoteCommandFromMenu = (command: NoteShortcut): void => {
  runner?.(command, 'menu')
}
