import { Note } from '../types/sequencer'
import {
  NoteTarget, clearNoteClipboard, noteClipboard, noteCommand, runNoteCommandFromMenu, setNoteCommandRunner,
} from './noteCommands'

const note = (pitch: number, start: number, length = 2): Note => ({ pitch, start, length, velocity: 0.8 })

const notes = [note(60, 4), note(64, 6), note(67, 12)]

const target = (over: Partial<NoteTarget> = {}): NoteTarget => ({
  notes, selected: new Set([0, 1]), limit: 16, pasteAt: 0, focused: true, span: null, playhead: 0, shortest: 1, ...over,
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

    // pasted into another pattern, past whose timeline's end the second copy
    // would start
    const other = [note(50, 0)]
    const pasted = noteCommand('paste', target({ notes: other, selected: new Set(), limit: 8, pasteAt: 6 }))
    expect(pasted?.notes).toEqual([note(50, 0), note(60, 6)])
    expect([...pasted?.selected ?? []]).toEqual([1])
  })

  it('repeat the selection where it ends, the copies selected', () => {
    const repeated = noteCommand('repeat', target())
    expect(repeated?.notes).toEqual([notes[0], notes[1], note(60, 8), note(64, 10), notes[2]])
    expect([...repeated?.selected ?? []]).toEqual([2, 3])
  })

  it('make nothing past the timeline’s end, leaving the selection as it was', () => {
    expect(noteCommand('repeat', target({ selected: new Set([2]), limit: 14 }))).toBeNull()
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
    // (and the time selection)
    expect(noteCommand('escape', target())).toEqual({ selected: new Set(), span: null })
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

describe('the note commands with a time selection', () => {
  // the notes at 4–6, 6–8 and 12–14; the span 5 to 13
  const span = { start: 5, end: 13 }
  const laid = (list: Note[] | undefined) => list?.map(n => [n.pitch, n.start, n.start + n.length])

  it('copy, cut and delete what is inside the span, cutting at its ends', () => {
    expect(noteCommand('copy', target({ span, selected: new Set() }))).toEqual({})
    expect(laid(noteClipboard())).toEqual([[60, 0, 1], [64, 1, 3], [67, 7, 8]])

    const cut = noteCommand('cut', target({ span, selected: new Set() }))
    expect(laid(cut?.notes)).toEqual([[60, 4, 5], [67, 13, 14]])
    expect(laid(noteCommand('delete', target({ span, selected: new Set() }))?.notes)).toEqual(laid(cut?.notes))
  })

  it('paste a copied span at the playhead, replacing what is there, and select it', () => {
    noteCommand('copy', target({ span: { start: 4, end: 8 }, selected: new Set() }))
    const pasted = noteCommand('paste', target({ playhead: 11, selected: new Set() }))
    // 11–15 replaced: the note at 12 goes
    expect(laid(pasted?.notes)).toEqual([[60, 4, 6], [64, 6, 8], [60, 11, 13], [64, 13, 15]])
    expect(pasted?.span).toEqual({ start: 11, end: 15 })
  })

  it('repeat the span straight after itself, and select the copy', () => {
    const repeated = noteCommand('repeat', target({ span: { start: 4, end: 8 }, selected: new Set() }))
    expect(laid(repeated?.notes)).toEqual([[60, 4, 6], [64, 6, 8], [60, 8, 10], [64, 10, 12], [67, 12, 14]])
    expect(repeated?.span).toEqual({ start: 8, end: 12 })
  })
})

describe('⌘B and the arrows in the Piano Roll', () => {
  it('cut the notes under the playhead', () => {
    const cut = noteCommand('blade', target({ playhead: 5, selected: new Set() }))
    expect(cut?.notes?.map(n => [n.start, n.length])).toEqual([[4, 1], [5, 1], [6, 2], [12, 2]])
  })

  it('move the selected notes a semitone, or an octave, keeping them selected', () => {
    const up = noteCommand('up', target())
    expect(up?.notes?.map(n => n.pitch)).toEqual([61, 65, 67])
    expect([...up?.selected ?? []]).toEqual([0, 1])
    expect(noteCommand('octave-down', target())?.notes?.map(n => n.pitch)).toEqual([48, 52, 67])
    expect(noteCommand('up', target({ selected: new Set() }))).toBeNull()
  })
})
