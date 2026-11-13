import {
  Note,
  UNCONFIRMED,
  clipContents,
  keepNotes,
  keepUnchanged,
  samePattern,
  sameRow,
  noteBars,
  patternBars,
  patternLabel,
  patternSeconds,
  rowName,
  rowNotes,
  sortNotes,
  stepAt,
  stepView,
  synthRowBySlot,
  withNotes,
  withStep,
  patternHint,
  searchPatterns,
  folderKey,
  folderIdOf,
  folderPathLabel,
  patternFolderInfos,
} from './sequencer'
import { pattern, sampleRow, step, steps, synthRow } from './sequencer.fixture'

const note = (pitch: number, start: number, length = 1, velocity = 0.8): Note => ({ pitch, start, length, velocity })

describe('patternLabel', () => {
  it('is the name, or "Sequence" and the number from the id', () => {
    expect(patternLabel({ id: 2, name: null })).toBe('Sequence 3')
    expect(patternLabel({ id: 2, name: 'verse' })).toBe('verse')
  })
})

describe('rowName', () => {
  it('is the label, or the sample without its folders and extension, or the synth', () => {
    expect(rowName(sampleRow(0, { path: '/drums/Kick 2.wav' }))).toBe('Kick 2')
    expect(rowName(sampleRow(0, { path: '/drums/kick.wav', label: 'Boom' }))).toBe('Boom')
    expect(rowName(synthRow(3, { name: 'DecentSampler 2' }))).toBe('DecentSampler 2')
    expect(rowName(synthRow(3, { label: 'Pad' }))).toBe('Pad')
  })
})

describe('synthRowBySlot', () => {
  const rows = [sampleRow(0), synthRow(2, { slot: 10 }), synthRow(10, { slot: 30 })]

  it('finds a synth row by its plugin’s slot, not its id', () => {
    expect(synthRowBySlot(rows, 10)?.id).toBe(2)
    expect(synthRowBySlot(rows, 30)?.id).toBe(10)
    expect(synthRowBySlot(rows, 2)).toBeUndefined()
    // a sample row has no slot
    expect(synthRowBySlot(rows, 0)).toBeUndefined()
  })
})

describe('sortNotes', () => {
  it('orders by start, then pitch, keeping notes alike in their order', () => {
    const quiet = note(60, 4, 1, 0.2)
    const loud = note(60, 4, 2, 1)
    const notes = [note(64, 4), loud, note(72, 0), quiet, note(60, 0.5)]
    expect(sortNotes(notes)).toEqual([note(72, 0), note(60, 0.5), loud, quiet, note(64, 4)])
  })

  it("doesn't change the list it was given", () => {
    const notes = [note(64, 4), note(60, 0)]
    sortNotes(notes)
    expect(notes).toEqual([note(64, 4), note(60, 0)])
  })
})

describe('rowNotes', () => {
  it('is the same empty list for a row without notes', () => {
    expect(rowNotes(pattern(0), 4)).toBe(rowNotes(pattern(1), 9))
    expect(rowNotes(pattern(0, { notes: { 4: steps(1) } }), 4)).toEqual(steps(1))
  })
})

describe('withNotes', () => {
  it('replaces one row’s notes, leaving out a row with none', () => {
    const p = pattern(0, { notes: { 1: steps(0), 2: steps(4) } })
    expect(withNotes(p, 2, steps(8)).notes).toEqual({ 1: steps(0), 2: steps(8) })
    expect(withNotes(p, 1, []).notes).toEqual({ 2: steps(4) })
    // the others' lists are the same lists
    expect(withNotes(p, 2, []).notes[1]).toBe(p.notes[1])
  })

  it('is unconfirmed until the engine answers', () => {
    expect(withNotes(pattern(0), 2, steps(8)).rev).toBe(UNCONFIRMED)
  })
})

describe('withStep', () => {
  const row = sampleRow(7, { stepPitch: 62 })

  it('lights a step as a 1-step note at the row’s step pitch, in order', () => {
    const lit = withStep(withStep(pattern(0), row, 8, true), row, 0, true)
    expect(lit.notes).toEqual({ 7: [step(0, 62), step(8, 62)] })
  })

  it('puts a lit step in order among the row’s other notes', () => {
    const p = pattern(0, { notes: { 7: [note(60, 4), note(64, 4)] } })
    expect(withStep(p, row, 4, true).notes[7]).toEqual([note(60, 4), step(4, 62), note(64, 4)])
  })

  it('counts any note at the step and pitch as lit, whatever its length or velocity', () => {
    const odd = note(62, 4, 3, 0.5)
    const p = pattern(0, { notes: { 7: [odd] } })
    expect(withStep(p, row, 4, true)).toBe(p)
  })

  it('leaves a pattern alone when the step is already as asked', () => {
    const p = pattern(0, { notes: { 7: [step(4, 62)] } })
    expect(withStep(p, row, 4, true)).toBe(p)
    expect(withStep(p, row, 5, false)).toBe(p)
  })

  it('clears every note at the step and pitch, whatever its length or velocity', () => {
    const p = pattern(0, { notes: { 7: [note(62, 4, 3, 0.2), step(4, 62), note(60, 4), step(8, 62)] } })
    expect(withStep(p, row, 4, false).notes).toEqual({ 7: [note(60, 4), step(8, 62)] })
    // a row with no notes left has no entry, like the engine
    expect(withStep(pattern(0, { notes: { 7: [step(4, 62)] } }), row, 4, false).notes).toEqual({})
  })

  it("doesn't change the pattern it was given", () => {
    const p = pattern(0, { notes: { 7: [step(4, 62)] } })
    withStep(p, row, 5, true)
    expect(p.notes).toEqual({ 7: [step(4, 62)] })
  })

  it('is unconfirmed until the engine answers', () => {
    expect(withStep(pattern(0), row, 4, true).rev).toBe(UNCONFIRMED)
    expect(withStep(pattern(0, { notes: { 7: [step(4, 62)] } }), row, 4, false).rev).toBe(UNCONFIRMED)
  })
})

describe('keepUnchanged', () => {
  const [a, b, c] = [pattern(0), pattern(1), pattern(2)]
  // the same patterns as the engine sends them again: new objects, same revs
  const again = (...patterns: typeof a[]) => patterns.map(p => ({ ...p, notes: { ...p.notes } }))

  it('keeps the very list when nothing in it changed', () => {
    const had = [a, b, c]
    expect(keepUnchanged(had, again(a, b, c), samePattern)).toBe(had)
  })

  it('takes a changed item, keeping the others', () => {
    const changed = pattern(1, { name: 'verse' })
    const kept = keepUnchanged([a, b, c], [...again(a), changed, ...again(c)], samePattern)
    expect(kept).toEqual([a, changed, c])
    expect(kept[0]).toBe(a)
    expect(kept[1]).toBe(changed)
    expect(kept[2]).toBe(c)
  })

  it('follows the order and the items sent, keeping what it can', () => {
    const d = pattern(3)
    const kept = keepUnchanged([a, b, c], [...again(c, a), d], samePattern)
    expect(kept).toEqual([c, a, d])
    expect(kept[0]).toBe(c)
    expect(kept[1]).toBe(a)
    expect(keepUnchanged([a, b], again(a), samePattern)).toEqual([a])
  })

  it('never keeps what the frontend changed itself', () => {
    const shown = withStep(a, sampleRow(7), 0, true)
    const confirmed = { ...shown, rev: a.rev + 1000 }
    expect(keepUnchanged([shown], [confirmed], samePattern)[0]).toBe(confirmed)
  })
})

describe('sameRow', () => {
  it('goes by rev, and for a synth by its plugin being missing or having an interface', () => {
    const row = synthRow(3)
    expect(sameRow(row, { ...row, label: 'not compared' })).toBe(true)
    expect(sameRow(row, { ...row, rev: row.rev + 1 })).toBe(false)
    expect(sameRow(row, { ...row, instrument: { ...row.instrument, missing: true } })).toBe(false)
    expect(sameRow(row, { ...row, instrument: { ...row.instrument, hasEditor: false } })).toBe(false)
    const sample = sampleRow(0)
    expect(sameRow(sample, { ...sample })).toBe(true)
  })
})

describe('keepNotes', () => {
  it('keeps the note lists of rows whose notes came back the same', () => {
    const had = pattern(0, { notes: { 1: steps(0, 4), 2: steps(8) } })
    const got = pattern(0, { notes: { 1: steps(0, 4), 2: steps(8, 12), 3: steps(1) } })
    const kept = keepNotes(had, got)
    expect(kept).toEqual(got)
    expect(kept.rev).toBe(got.rev)
    expect(kept.notes[1]).toBe(had.notes[1])
    expect(kept.notes[2]).toBe(got.notes[2])
    // a velocity or length that differs is a change
    const louder = pattern(0, { notes: { 1: [{ ...step(0), velocity: 0.5 }, step(4)] } })
    expect(keepNotes(had, louder).notes[1]).toBe(louder.notes[1])
  })
})

describe('stepView', () => {
  const oneShot = sampleRow(0)
  const gated = sampleRow(0, { gated: true })
  const synth = synthRow(0)

  it('lists the steps lit, in order, or none', () => {
    expect(stepView(oneShot, steps(0, 4, 12), 16)).toEqual([0, 4, 12])
    expect(stepView(oneShot, [], 16)).toEqual([])
  })

  it('lights a step once for two notes on it, whatever their velocity', () => {
    expect(stepView(synth, [step(4), { ...step(4), velocity: 0.3 }], 16)).toEqual([4])
  })

  it('is notes for a note off the grid, or at another pitch', () => {
    expect(stepView(oneShot, [step(0), note(60, 2.5)], 16)).toBeNull()
    expect(stepView(oneShot, [step(0), step(4, 64)], 16)).toBeNull()
    expect(stepView(sampleRow(0, { stepPitch: 64 }), [step(4, 64)], 16)).toEqual([4])
  })

  it('takes notes of any length on a one-shot sample row only', () => {
    const long = note(60, 4, 3)
    expect(stepView(oneShot, [long], 16)).toEqual([4])
    expect(stepView(gated, [long], 16)).toBeNull()
    expect(stepView(synth, [long], 16)).toBeNull()
  })

  it('leaves out notes past the end, whatever they are', () => {
    expect(stepView(synth, [step(4), note(72, 17.5, 3)], 16)).toEqual([4])
  })
})

describe('patternBars', () => {
  it('counts bars of the time signature', () => {
    expect(patternBars(pattern(0, { length: 32 }), 4)).toBe(2)
    expect(patternBars(pattern(0, { length: 24 }), 3)).toBe(2)
    // set in 4/4, now in 3/4
    expect(patternBars(pattern(0, { length: 16 }), 3)).toBeNull()
  })
})

describe('patternSeconds', () => {
  it('is the steps as 16th notes at the tempo', () => {
    expect(patternSeconds(pattern(0, { length: 16 }), 120)).toBe(2)
    expect(patternSeconds(pattern(0, { length: 64 }), 60)).toBe(16)
  })
})

describe('stepAt', () => {
  it('follows the run, wrapping at the end', () => {
    const p = pattern(0, { length: 16 })
    expect(stepAt(0, p, 500)).toBe(0)
    expect(stepAt(130, p, 500)).toBe(1)
    expect(stepAt(2_000, p, 500)).toBe(0)
    expect(stepAt(2_130, p, 500)).toBe(1)
    expect(stepAt(100, p, 0)).toBeNull()
  })
})

describe('clipContents', () => {
  it("gives each sample row a lane of its notes' starts inside the pattern, in row order", () => {
    const rows = [sampleRow(5), synthRow(1), sampleRow(3)]
    const p = pattern(0, { notes: { 3: [step(1), step(20)], 1: [note(72, 2, 4)], 9: steps(2) } })
    expect(clipContents(p, rows)).toEqual({ lanes: [[], [1]], notes: [note(72, 2, 4)] })
  })

  it("draws every synth row's notes as bars: those starting in the clip's part, cut at its end", () => {
    const rows = [synthRow(1), sampleRow(0), synthRow(2)]
    const p = pattern(0, { notes: { 1: [note(60, 0, 4), note(64, 6, 8)], 0: steps(0), 2: [note(72, 12, 2)] } })
    const { notes } = clipContents(p, rows)
    expect(noteBars(notes, p.length, 0, 16)).toEqual({
      rows: 13,
      bars: [{ start: 0, end: 4, row: 12 }, { start: 6, end: 14, row: 8 }, { start: 12, end: 14, row: 0 }],
    })
    // trimmed to steps 4 to 10: the first note began before it, the second is cut short
    expect(noteBars(notes, p.length, 4, 10).bars).toEqual([{ start: 6, end: 10, row: 5 }])
    expect(noteBars(clipContents(pattern(0), rows).notes, 16, 0, 16)).toEqual({ rows: 0, bars: [] })
  })
})

describe('noteBars', () => {
  it('draws the notes started in the part played, cut at its end, with their range in the middle', () => {
    const { rows, bars } = noteBars([note(60, 0, 4), note(62, 6, 4), note(64, 12)], 16, 0, 8)
    expect(rows).toBe(12)
    expect(bars).toEqual([{ start: 0, end: 4, row: 6 }, { start: 6, end: 8, row: 4 }])
  })
})

const named = (id: number, name: string | null, length = 16) => pattern(id, { name, length })

describe('searchPatterns', () => {
  const patterns = [named(0, 'Verse beat'), named(1, null), named(2, 'Chorus'), named(11, 'verse fill')]

  it('keeps every pattern, in order, without a search', () => {
    expect(searchPatterns(patterns, '  ')).toEqual(patterns)
  })

  it('finds names without minding case', () => {
    expect(searchPatterns(patterns, 'VERSE').map(p => p.id)).toEqual([0, 11])
  })

  it('finds unnamed patterns by their number', () => {
    expect(searchPatterns(patterns, '2').map(p => p.id)).toEqual([1])
  })
})

describe('patternHint', () => {
  it('gives the length in bars', () => {
    expect(patternHint(named(0, null, 16), 4, 0)).toBe('1 bar')
    expect(patternHint(named(0, null, 32), 4, 0)).toBe('2 bars')
  })

  it('gives steps when the bars are not whole', () => {
    expect(patternHint(named(0, null, 16), 3, 0)).toBe('16 steps')
  })

  it('counts the clips in the playlist', () => {
    expect(patternHint(named(0, null, 16), 4, 3)).toBe('1 bar · 3×')
  })
})

describe('pattern folders', () => {
  const folders = [{ id: 3, name: 'Drums' }, { id: 7, name: 'Fills', parent: 3 }]

  it('keys folders by id, with the top as the empty key', () => {
    expect(folderKey(7)).toBe('7')
    expect(folderKey(null)).toBe('')
    expect(folderKey(undefined)).toBe('')
    expect(folderIdOf('7')).toBe(7)
    expect(folderIdOf('')).toBeNull()
    expect(patternFolderInfos(folders)).toEqual([
      { key: '3', name: 'Drums', parent: '' },
      { key: '7', name: 'Fills', parent: '3' },
    ])
  })

  it('names where a folder is, from the top', () => {
    expect(folderPathLabel(folders, 7)).toBe('Drums / Fills')
    expect(folderPathLabel(folders, null)).toBe('')
    expect(folderPathLabel([{ id: 1, name: 'a', parent: 2 }, { id: 2, name: 'b', parent: 1 }], 1)).toBe('b / a')
  })
})
