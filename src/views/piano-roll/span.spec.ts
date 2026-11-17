import { Note } from '../../types/sequencer'
import { bladeNotes, clearSpan, copySpan, cutAtEnds, pasteSpan, repeatSpan, transposeNotes } from './span'

const note = (start: number, length: number, pitch = 60): Note => ({ pitch, start, length, velocity: 0.8 })
// as [start, end, pitch], in order
const laid = (notes: Note[]) => notes.map(n => [n.start, n.start + n.length, n.pitch])
  .sort((a, b) => a[0] - b[0] || a[2] - b[2])

describe('the time selection in the Piano Roll', () => {
  // steps 0–8 at middle C, 8–16 at D, and 4–12 at E
  const notes = [note(0, 8), note(8, 8, 62), note(4, 8, 64)]

  it('cuts notes across its ends, and knows which are inside', () => {
    const { notes: cut, inside } = cutAtEnds(notes, { start: 6, end: 10 })
    expect(laid(inside.map(i => cut[i]))).toEqual([[6, 8, 60], [6, 10, 64], [8, 10, 62]])
    // (E is cut at both ends: three pieces)
    expect(cut).toHaveLength(7)
  })

  it('clears what is inside, leaving the rest where it was', () => {
    expect(laid(clearSpan(notes, { start: 6, end: 10 }))).toEqual([
      [0, 6, 60], [4, 6, 64], [10, 16, 62], [10, 12, 64],
    ])
    // nothing inside: the same notes
    expect(clearSpan(notes, { start: 20, end: 24 })).toBe(notes)
  })

  it('copies what is inside, from the span\'s start', () => {
    const copied = copySpan(notes, { start: 6, end: 10 })
    expect(copied.length).toBe(4)
    expect(laid(copied.notes)).toEqual([[0, 2, 60], [0, 4, 64], [2, 4, 62]])
  })

  it('pastes a span over what is there, replacing it, and says where it went', () => {
    const copied = { notes: [note(1, 2, 70)], length: 4 }
    const pasted = pasteSpan(notes, copied, 2, 64)
    expect(pasted?.span).toEqual({ start: 2, end: 6 })
    expect(laid(pasted!.notes)).toEqual([[0, 2, 60], [3, 5, 70], [6, 8, 60], [6, 12, 64], [8, 16, 62]])
    // not past where time ends, and kept short of it
    expect(pasteSpan(notes, copied, 64, 64)).toBeNull()
    expect(pasteSpan([], copied, 62, 64)?.span).toEqual({ start: 62, end: 64 })
  })

  it('repeats a span straight after itself', () => {
    const repeated = repeatSpan([note(0, 4)], { start: 0, end: 4 }, 64)
    expect(repeated?.span).toEqual({ start: 4, end: 8 })
    expect(laid(repeated!.notes)).toEqual([[0, 4, 60], [4, 8, 60]])
  })
})

describe('↑ and ↓', () => {
  it('move the selected notes, keeping a chord\'s shape at the piano\'s ends', () => {
    const notes = [note(0, 4, 60), note(0, 4, 64), note(4, 4, 67)]
    expect(transposeNotes(notes, [0, 1], 1)?.map(n => n.pitch)).toEqual([61, 65, 67])
    expect(transposeNotes(notes, [0, 1], -12)?.map(n => n.pitch)).toEqual([48, 52, 67])
    // only 3 semitones to the top: as far as that, for both
    const high = [note(0, 4, 120), note(0, 4, 124)]
    expect(transposeNotes(high, [0, 1], 12)?.map(n => n.pitch)).toEqual([123, 127])
    expect(transposeNotes([note(0, 4, 127)], [0], 1)).toBeNull()
    expect(transposeNotes(notes, [], 1)).toBeNull()
  })
})

describe('⌘B in the Piano Roll', () => {
  const notes = [note(0, 8), note(0, 8, 64), note(8, 8)]

  it('cuts every note under the playhead, with none selected', () => {
    expect(laid(bladeNotes(notes, new Set(), 4, 1)!)).toEqual([[0, 4, 60], [0, 4, 64], [4, 8, 60], [4, 8, 64], [8, 16, 60]])
  })

  it('cuts just the selected ones, with some selected', () => {
    expect(laid(bladeNotes(notes, new Set([1]), 4, 1)!)).toEqual([[0, 8, 60], [0, 4, 64], [4, 8, 64], [8, 16, 60]])
    // a selected note the playhead isn't on: nothing
    expect(bladeNotes(notes, new Set([2]), 4, 1)).toBeNull()
  })

  it('cuts nothing on an edge, or too near one', () => {
    expect(bladeNotes(notes, new Set(), 8, 1)).toBeNull()
    expect(bladeNotes(notes, new Set(), 0.5, 1)).toBeNull()
  })
})
