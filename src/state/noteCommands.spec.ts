import { Note } from '../types/sequencer'
import {
  NoteTarget, clearNoteClipboard, noteClipboard, noteCommand, runNoteCommandFromMenu, setNoteCommandRunner,
} from './noteCommands'

const note = (pitch: number, start: number, length = 2): Note => ({ pitch, start, length, velocity: 0.8 })

const notes = [note(60, 4), note(64, 6), note(67, 12)]

const target = (over: Partial<NoteTarget> = {}): NoteTarget => ({
  notes, selected: new Set([0, 1]), patternLength: 16, pasteAt: 0, focused: true, ...over,
})

beforeEach(() => clearNoteClipboard())

describe('the note commands', () => {
  it('do nothing unless the Piano Roll is focused', () => {
    expect(noteCommand('copy', target({ focused: false }))).toBeNull()
    expect(noteClipboard()).toEqual([])
    expect(noteCommand('select-all', target({ focused: false }))).toBeNull()
  })

  it('copy the selection, and paste it with the earliest at the step given', () => {
    expect(noteCommand('copy', target())).toEqual({})
    expect(noteClipboard()).toEqual([notes[0], notes[1]])

    const pasted = noteCommand('paste', target({ pasteAt: 8, selected: new Set() }))
    // in order, by start: the copies before the note at 12
    expect(pasted?.notes).toEqual([notes[0], notes[1], note(60, 8), note(64, 10), notes[2]])
    // the copies become the selection, where they went
    expect([...pasted?.selected ?? []]).toEqual([2, 3])
  })

  it('cut the selection to the clipboard, and keep it across patterns', () => {
    const cut = noteCommand('cut', target())
    expect(cut?.notes).toEqual([notes[2]])
    expect(cut?.selected?.size).toBe(0)

    // pasted into another pattern, past whose end the second copy would start
    const other = [note(50, 0)]
    const pasted = noteCommand('paste', target({ notes: other, selected: new Set(), patternLength: 8, pasteAt: 6 }))
    expect(pasted?.notes).toEqual([note(50, 0), note(60, 6)])
    expect([...pasted?.selected ?? []]).toEqual([1])
  })

  it('repeat the selection where it ends, the copies selected', () => {
    const repeated = noteCommand('repeat', target())
    expect(repeated?.notes).toEqual([notes[0], notes[1], note(60, 8), note(64, 10), notes[2]])
    expect([...repeated?.selected ?? []]).toEqual([2, 3])
  })

  it('make nothing past the pattern’s end, leaving the selection as it was', () => {
    expect(noteCommand('repeat', target({ selected: new Set([2]), patternLength: 14 }))).toBeNull()
    expect(noteCommand('copy', target())).toEqual({})
    expect(noteCommand('paste', target({ pasteAt: 16 }))).toBeNull()
  })

  it('do nothing to an empty selection or clipboard', () => {
    const none = target({ selected: new Set() })
    expect(noteCommand('copy', none)).toBeNull()
    expect(noteCommand('cut', none)).toBeNull()
    expect(noteCommand('repeat', none)).toBeNull()
    expect(noteCommand('delete', none)).toBeNull()
    expect(noteCommand('paste', none)).toBeNull()
  })

  it('select all, delete the selection, and clear it with Escape', () => {
    expect([...noteCommand('select-all', target())?.selected ?? []]).toEqual([0, 1, 2])
    expect(noteCommand('delete', target())).toEqual({ notes: [notes[2]], selected: new Set() })
    expect(noteCommand('escape', target())).toEqual({ selected: new Set() })
  })

  it('reach the Piano Roll from the menu through its runner', () => {
    const run = vi.fn()
    setNoteCommandRunner(run)
    runNoteCommandFromMenu('repeat')
    expect(run).toHaveBeenCalledWith('repeat', 'menu')
    setNoteCommandRunner(null)
    runNoteCommandFromMenu('repeat')
    expect(run).toHaveBeenCalledTimes(1)
  })
})
