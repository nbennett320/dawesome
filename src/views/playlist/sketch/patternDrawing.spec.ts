import { Note } from '../../../types/sequencer'
import { pattern, sampleRow, step, synthRow } from '../../../types/sequencer.fixture'
import { firstFrom, patternDrawing } from './patternDrawing'

const note = (pitch: number, start: number, length = 1): Note => ({ pitch, start, length, velocity: 0.8 })

describe('patternDrawing', () => {
  const rows = [sampleRow(0), synthRow(1), synthRow(2)]
  const drums = pattern(0, { notes: { 0: [step(0), step(8), step(20)], 1: [note(64, 6, 12)], 2: [note(60, 2, 4)] } })

  it('is a lane per sample row, and the synth rows\' notes as bars in order of their starts, cut at the end', () => {
    expect(patternDrawing(drums, rows)).toEqual({
      lanes: [[0, 8]],
      bars: [{ start: 2, end: 6, row: 7 }, { start: 6, end: 16, row: 3 }],
      rows: 12,
    })
  })

  it('is worked out once for a pattern and its rows', () => {
    const first = patternDrawing(drums, rows)
    expect(patternDrawing(drums, rows)).toBe(first)
  })

  it('is worked out again for a changed pattern (a new object), or for changed rows', () => {
    const first = patternDrawing(drums, rows)
    const edited = { ...drums, notes: { ...drums.notes, 0: [step(4)] } }
    expect(patternDrawing(edited, rows)).not.toBe(first)
    expect(patternDrawing(edited, rows).lanes).toEqual([[4]])

    // the rows moved: the same pattern, with its lanes in their new order
    const moved = [synthRow(1), sampleRow(3), synthRow(2), sampleRow(0)]
    expect(patternDrawing(drums, moved)).not.toBe(first)
    expect(patternDrawing(drums, moved).lanes).toEqual([[], [0, 8]])
  })
})

describe('firstFrom', () => {
  it('finds the first item starting at or after a step', () => {
    const starts = [0, 2, 2, 5, 9]
    const at = (step: number) => firstFrom(starts, start => start, step)
    expect([-1, 0, 1, 2, 3, 9, 10].map(at)).toEqual([0, 0, 1, 1, 3, 4, 5])
    expect(firstFrom([], start => start, 3)).toBe(0)
  })
})
