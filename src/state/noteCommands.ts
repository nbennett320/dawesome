// the Piano Roll's commands, run the same way wherever they came from: a
// key, the Edit menu, or the system's copy, cut and paste events. like the
// clips' (see clipCommands.ts), each does nothing unless its view is the
// focused one
import { Note } from '../types/sequencer'
import {
  allIndices, copyNotes, deleteNotes, followPlaces, pasteNotes, repeatNotes, sortedWithPlaces,
} from '../views/piano-roll/notes'

// the commands with a ⌘ shortcut, which can arrive by more than one route
// (the key, the menu, the system's clipboard events; see sameCommand), and
// those only a key gives
export type NoteShortcut = 'copy' | 'cut' | 'paste' | 'repeat' | 'select-all'
export type NoteCommand = NoteShortcut | 'delete' | 'escape'

// the notes copied: the page's own clipboard (not the system's, nor the
// clips'), kept as another pattern or synth is shown
let clipboard: Note[] = []

export const noteClipboard = (): Note[] => clipboard

// for tests
export const clearNoteClipboard = (): void => {
  clipboard = []
}

export interface NoteTarget {
  // the row's notes in the pattern shown, in order, and which are selected
  notes: Note[]
  selected: ReadonlySet<number>
  // the pattern's length, in steps: no copy starts at or past it
  patternLength: number
  // the step a paste puts the earliest note at (see pasteStep)
  pasteAt: number
  // whether the Piano Roll is the focused view
  focused: boolean
}

// whether a command takes notes away, which trims the bars it leaves empty
// at the pattern's end (see trimmedLength)
export const clearsNotes = (command: NoteCommand): boolean => command === 'delete' || command === 'cut'

// what a command changed: the notes (in order), and what's selected after
export interface NoteOutcome {
  notes?: Note[]
  selected?: ReadonlySet<number>
}

// a command's notes put in order, the selection following them
const inOrder = (outcome: NoteOutcome, selected: ReadonlySet<number>): NoteOutcome => {
  if(!outcome.notes) return outcome
  const sorted = sortedWithPlaces(outcome.notes)
  return { notes: sorted.notes, selected: followPlaces(outcome.selected ?? selected, sorted.at) }
}

// what a command does to the notes, before they're put in order
const changeNotes = (command: NoteCommand, target: NoteTarget): NoteOutcome | null => {
  if(!target.focused) return null
  const { notes, selected, patternLength, pasteAt } = target
  const none = new Set<number>()

  switch(command) {
    case 'copy':
      if(!selected.size) return null
      clipboard = copyNotes(notes, selected)
      return {}
    case 'cut':
      if(!selected.size) return null
      clipboard = copyNotes(notes, selected)
      return { notes: deleteNotes(notes, selected), selected: none }
    case 'paste': {
      if(!clipboard.length) return null
      const pasted = pasteNotes(notes, clipboard, pasteAt, patternLength)
      return pasted.added.length ? { notes: pasted.notes, selected: new Set(pasted.added) } : null
    }
    case 'repeat': {
      if(!selected.size) return null
      const repeated = repeatNotes(notes, selected, patternLength)
      return repeated.added.length ? { notes: repeated.notes, selected: new Set(repeated.added) } : null
    }
    case 'select-all': return { selected: allIndices(notes) }
    case 'delete': return selected.size ? { notes: deleteNotes(notes, selected), selected: none } : null
    case 'escape': return { selected: none }
    default: return null
  }
}

// run a command on the notes in `target`. null when it does nothing: the
// Piano Roll isn't focused, nothing's selected to copy, cut, repeat or
// delete, there's nothing to paste, or every copy would be past the
// pattern's end (and what's selected stays as it was). copy and cut fill
// the clipboard
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
