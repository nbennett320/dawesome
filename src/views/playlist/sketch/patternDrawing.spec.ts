import { Note } from '../../../types/sequencer'
import { pattern, sampleRow, step, synthRow } from '../../../types/sequencer.fixture'
import { firstFrom, patternLanes } from './patternDrawing'

const note = (pitch: number, start: number, length = 1): Note => ({ pitch, start, length, velocity: 0.8 })

describe('patternLanes', () => {
  const rows = [sampleRow(0), synthRow(1), sampleRow(3), synthRow(2), sampleRow(4)]
  // row 3 plays nothing inside the pattern, and row 4 nothing at all
  const mixed = pattern(0, {
    notes: { 0: [step(0), step(8), step(20)], 1: [note(64, 6, 12)], 2: [note(60, 2, 4), note(62, 3)], 3: [step(16)] },
  })

  it('is a lane per row that plays notes, in the rows\' order: ticks for a sample row, bars for a synth row', () => {
    expect(patternLanes(mixed, rows)).toEqual([
      { kind: 'sample', ticks: [0, 8] },
      // cut at the pattern's end
      { kind: 'synth', bars: [{ start: 6, end: 16, row: 5 }], pitches: 12 },
      { kind: 'synth', bars: [{ start: 2, end: 6, row: 6 }, { start: 3, end: 4, row: 4 }], pitches: 12 },
    ])
  })

  it('ticks every note of a sample row, whatever its pitch or length', () => {
    const played = pattern(0, { notes: { 0: [note(48, 0, 6), note(72, 0.5), step(4)] } })
    expect(patternLanes(played, rows)).toEqual([{ kind: 'sample', ticks: [0, 0.5, 4] }])
  })

  it('is the lit steps of each row when every row’s notes are plain steps, as the sequencer shows them', () => {
    // (a one-shot sample row's steps may be any length; the synth's are a step long)
    const beat = pattern(0, { notes: { 0: [step(0), { ...step(8), length: 3 }], 1: [step(4)], 3: [step(16)] } })
    expect(patternLanes(beat, rows)).toEqual([{ kind: 'steps', steps: [0, 8] }, { kind: 'steps', steps: [4] }])
  })

  it('is worked out again when a row’s step pitch or gate changes what its notes are', () => {
    const beat = pattern(0, { notes: { 0: [step(0), { ...step(8), length: 3 }], 1: [step(4)] } })
    expect(patternLanes(beat, rows)[0].kind).toBe('steps')
    // gated, a 3-step note isn't a plain step: the notes' look for every row
    const gated = rows.map(row => (row.id === 0 && row.instrument.kind === 'sample'
      ? { ...row, instrument: { ...row.instrument, gated: true } } : row))
    expect(patternLanes(beat, gated).map(lane => lane.kind)).toEqual(['sample', 'synth'])
    // the synth's step pitch moved: its notes aren't at it any more
    const repitched = rows.map(row => (row.id === 1 ? { ...row, stepPitch: 62 } : row))
    expect(patternLanes(beat, repitched).map(lane => lane.kind)).toEqual(['sample', 'synth'])
  })

  it('is worked out once for a pattern and its rows', () => {
    const first = patternLanes(mixed, rows)
    expect(patternLanes(mixed, rows)).toBe(first)
  })

  it('is worked out again for a changed pattern (a new object), or for changed rows', () => {
    const first = patternLanes(mixed, rows)
    const edited = { ...mixed, notes: { ...mixed.notes, 0: [step(4)] } }
    expect(patternLanes(edited, rows)).not.toBe(first)
    expect(patternLanes(edited, rows)[0]).toEqual({ kind: 'sample', ticks: [4] })

    // the rows moved: the same pattern, with its lanes in their new order
    const moved = [synthRow(2), sampleRow(0), synthRow(1)]
    expect(patternLanes(mixed, moved)).not.toBe(first)
    expect(patternLanes(mixed, moved).map(lane => lane.kind)).toEqual(['synth', 'sample', 'synth'])
  })

  it('is kept while the rows change only in ways it doesn\'t draw (a knob, a mute, a name)', () => {
    const first = patternLanes(mixed, rows)
    const turned = rows.map(row => (row.id === 0 ? { ...row, volume: 0.5, muted: true, label: 'Kick' } : row))
    expect(patternLanes(mixed, turned)).toBe(first)
    expect(patternLanes(mixed, [...turned])).toBe(first)
  })

  it('is worked out again when a row moves, or plays another kind of instrument', () => {
    const first = patternLanes(mixed, rows)
    const swapped = [rows[1], rows[0], ...rows.slice(2)]
    expect(patternLanes(mixed, swapped)).not.toBe(first)
    expect(patternLanes(mixed, swapped).map(lane => lane.kind)).toEqual(['synth', 'sample', 'synth'])

    const back = patternLanes(mixed, rows)
    // (row 0 playing a synth: its notes are bars)
    const replaced = rows.map(row => (row.id === 0 ? synthRow(0) : row))
    expect(patternLanes(mixed, replaced)).not.toBe(back)
    expect(patternLanes(mixed, replaced)[0].kind).toBe('synth')
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
