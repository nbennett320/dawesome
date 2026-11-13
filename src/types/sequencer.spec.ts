import {
  Note,
  UNCONFIRMED,
  addedEnd,
  barsReached,
  clipPasses,
  greyedSteps,
  grownLength,
  keepNotes,
  keepUnchanged,
  samePattern,
  sameRow,
  noteBars,
  patternBars,
  patternHolds,
  patternLabel,
  patternSeconds,
  patternStepAt,
  repeatLines,
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

// steps in a 4/4 bar
const BAR = 16

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
    expect(withNotes(p, 2, steps(8), BAR).notes).toEqual({ 1: steps(0), 2: steps(8) })
    expect(withNotes(p, 1, [], BAR).notes).toEqual({ 2: steps(4) })
    // the others' lists are the same lists
    expect(withNotes(p, 2, [], BAR).notes[1]).toBe(p.notes[1])
  })

  it('is unconfirmed until the engine answers', () => {
    expect(withNotes(pattern(0), 2, steps(8), BAR).rev).toBe(UNCONFIRMED)
  })

  it('grows the pattern to fit the notes added, as the engine does', () => {
    expect(withNotes(pattern(0), 2, [note(60, 30, 4)], BAR).length).toBe(48)
    expect(withNotes(pattern(0), 2, [note(60, 12, 4)], BAR).length).toBe(16)
    // in 3/4
    expect(withNotes(pattern(0, { length: 12 }), 2, [note(60, 12)], 12).length).toBe(24)
    // never shrinking it, nor raising its floor
    const long = withNotes(pattern(0, { length: 64 }), 2, [note(60, 0)], BAR)
    expect([long.length, long.floor]).toEqual([64, 64])
  })

  it('doesn’t grow back for notes kept past an end made shorter', () => {
    // 2 bars once, 1 bar now, a note in each
    const p = pattern(0, { length: 16, notes: { 2: [note(60, 0), note(60, 20)] } })
    expect(withNotes(p, 2, [note(60, 2), note(60, 20)], BAR).length).toBe(16)
    expect(withNotes(p, 2, [note(60, 0), note(60, 24)], BAR).length).toBe(32)
  })
})

describe('grownLength', () => {
  it('reaches the end of the bar holding the end, never shorter, never past 128 bars', () => {
    expect(grownLength(16, 16, BAR)).toBe(16)
    expect(grownLength(16, 16.5, BAR)).toBe(32)
    expect(grownLength(48, 1, BAR)).toBe(48)
    expect(grownLength(16, 5000, BAR)).toBe(2048)
    expect(grownLength(3000, 1, BAR)).toBe(3000)
    expect(grownLength(12, 13, 12)).toBe(24)
  })
})

describe('addedEnd', () => {
  const [a, b, c] = [note(60, 0), note(62, 4, 4), note(64, 6)]

  it('is where the notes added end, the furthest, or 0 for none', () => {
    expect(addedEnd([a, b], [a, b])).toBe(0)
    expect(addedEnd([a, b], [a])).toBe(0)
    expect(addedEnd([a], [a, b, c])).toBe(8)
    expect(addedEnd([a, b, a], [a, c, a])).toBe(7)
  })

  it('counts a note changed in any way, and a second copy of one, as added', () => {
    expect(addedEnd([b], [{ ...b, velocity: 0.1 }])).toBe(8)
    expect(addedEnd([c], [c, c])).toBe(7)
    expect(addedEnd([c, c], [c, c])).toBe(0)
  })
})

describe('withStep', () => {
  const row = sampleRow(7, { stepPitch: 62 })

  it('lights a step as a 1-step note at the row’s step pitch, in order', () => {
    const lit = withStep(withStep(pattern(0), row, 8, true, BAR), row, 0, true, BAR)
    expect(lit.notes).toEqual({ 7: [step(0, 62), step(8, 62)] })
  })

  it('puts a lit step in order among the row’s other notes', () => {
    const p = pattern(0, { notes: { 7: [note(60, 4), note(64, 4)] } })
    expect(withStep(p, row, 4, true, BAR).notes[7]).toEqual([note(60, 4), step(4, 62), note(64, 4)])
  })

  it('counts any note at the step and pitch as lit, whatever its length or velocity', () => {
    const odd = note(62, 4, 3, 0.5)
    const p = pattern(0, { notes: { 7: [odd] } })
    expect(withStep(p, row, 4, true, BAR)).toBe(p)
  })

  it('leaves a pattern alone when the step is already as asked', () => {
    const p = pattern(0, { notes: { 7: [step(4, 62)] } })
    expect(withStep(p, row, 4, true, BAR)).toBe(p)
    expect(withStep(p, row, 5, false, BAR)).toBe(p)
  })

  it('clears every note at the step and pitch, whatever its length or velocity', () => {
    const p = pattern(0, { notes: { 7: [note(62, 4, 3, 0.2), step(4, 62), note(60, 4), step(8, 62)] } })
    expect(withStep(p, row, 4, false, BAR).notes).toEqual({ 7: [note(60, 4), step(8, 62)] })
    // a row with no notes left has no entry, like the engine
    expect(withStep(pattern(0, { notes: { 7: [step(4, 62)] } }), row, 4, false, BAR).notes).toEqual({})
  })

  it("doesn't change the pattern it was given", () => {
    const p = pattern(0, { notes: { 7: [step(4, 62)] } })
    withStep(p, row, 5, true, BAR)
    expect(p.notes).toEqual({ 7: [step(4, 62)] })
  })

  it('is unconfirmed until the engine answers', () => {
    expect(withStep(pattern(0), row, 4, true, BAR).rev).toBe(UNCONFIRMED)
    expect(withStep(pattern(0, { notes: { 7: [step(4, 62)] } }), row, 4, false, BAR).rev).toBe(UNCONFIRMED)
  })

  it('grows the pattern to the end of a lit step’s bar, the bars between empty', () => {
    const grown = withStep(pattern(0), row, 37, true, BAR)
    expect([grown.length, grown.floor]).toEqual([48, 16])
    expect(grown.notes[7]).toEqual([step(37, 62)])
    expect(withStep(pattern(0), row, 30, true, 12).length).toBe(36)
  })

  it('grows for a step lit already, past an end made shorter, and not for one cleared', () => {
    const p = pattern(0, { notes: { 7: [step(20, 62)] } })
    const grown = withStep(p, row, 20, true, BAR)
    expect([grown.length, grown.rev]).toEqual([32, UNCONFIRMED])
    expect(grown.notes).toBe(p.notes)
    const cleared = withStep(p, row, 20, false, BAR)
    expect([cleared.length, cleared.notes]).toEqual([16, {}])
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
    const shown = withStep(a, sampleRow(7), 0, true, BAR)
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

  it('shows a synth row’s 1-step notes at its step pitch as steps, and a chord as notes', () => {
    expect(stepView(synth, steps(0, 8), 16)).toEqual([0, 8])
    expect(stepView(synth, [step(0), step(0, 64)], 16)).toBeNull()
  })
})

describe('greyedSteps', () => {
  it('is a bar past the end, or what’s left of the longest a pattern can be', () => {
    expect(greyedSteps(16, BAR)).toBe(16)
    expect(greyedSteps(12, 12)).toBe(12)
    expect(greyedSteps(2040, BAR)).toBe(8)
    expect(greyedSteps(2048, BAR)).toBe(0)
    // longer than 128 bars of the time signature now
    expect(greyedSteps(2048, 12)).toBe(0)
  })
})

describe('barsReached', () => {
  it('counts the bars a pattern reaches into', () => {
    expect(barsReached(pattern(0, { length: 32 }), 4)).toBe(2)
    expect(barsReached(pattern(0, { length: 16 }), 3)).toBe(2)
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

describe('patternStepAt', () => {
  it('plays the pattern end to end, over and over, from where the clip starts in it', () => {
    expect(patternStepAt(0, 0, 16)).toBe(0)
    expect(patternStepAt(0, 15.5, 16)).toBe(15.5)
    expect(patternStepAt(0, 16, 16)).toBe(0)
    expect(patternStepAt(4, 13, 16)).toBe(1)
    // a clip starting on the pattern's third time round (not wrapped)
    expect(patternStepAt(36, 0, 16)).toBe(4)
    expect(patternStepAt(36, 30, 16)).toBe(2)
    // and before its start
    expect(patternStepAt(0, -1, 16)).toBe(15)
  })
})

describe('clipPasses', () => {
  it('is each time through the pattern: the part played, and where it starts', () => {
    // all of it once
    expect(clipPasses(0, 16, 16)).toEqual([{ from: 0, to: 16, at: 0 }])
    // part of it
    expect(clipPasses(4, 8, 16)).toEqual([{ from: 4, to: 12, at: 0 }])
    // from step 4, round it two and a quarter times
    expect(clipPasses(4, 36, 16)).toEqual([
      { from: 4, to: 16, at: 0 }, { from: 0, to: 16, at: 16 }, { from: 0, to: 8, at: 32 },
    ])
    // the second half of a clip cut on its pattern's second time round
    expect(clipPasses(20.5, 15.5, 16)).toEqual([{ from: 4.5, to: 16, at: 16 }, { from: 0, to: 4, at: 32 }])
    expect(clipPasses(0, 16, 0)).toEqual([])
  })
})

describe('repeatLines', () => {
  it('is where the pattern starts over, in steps from the clip\'s start', () => {
    expect(repeatLines(0, 16, 16)).toEqual([])
    expect(repeatLines(0, 64, 16)).toEqual([16, 32, 48])
    expect(repeatLines(4, 36, 16)).toEqual([12, 28])
    expect(repeatLines(20.5, 15.5, 16)).toEqual([11.5])
    // ending where it would start over: no line on its edge
    expect(repeatLines(8, 24, 16)).toEqual([8])
  })
})

describe('patternHolds', () => {
  const kinds = new Map([[0, 'sample' as const], [1, 'synth' as const], [2, 'sample' as const]])

  it('says which kinds of instrument a pattern has notes for, past its end too', () => {
    expect(patternHolds(pattern(0, { notes: { 0: steps(0), 2: steps(4) } }), kinds)).toEqual({ samples: true, synths: false })
    expect(patternHolds(pattern(0, { notes: { 1: [note(60, 40)] } }), kinds)).toEqual({ samples: false, synths: true })
    expect(patternHolds(pattern(0, { notes: { 0: steps(0), 1: [note(60, 0)] } }), kinds)).toEqual({ samples: true, synths: true })
  })

  it('leaves out empty lists and rows it doesn\'t know', () => {
    expect(patternHolds(pattern(0, { notes: { 1: [], 9: steps(0) } }), kinds)).toEqual({ samples: false, synths: false })
  })
})

describe('noteBars', () => {
  it('draws the notes started in the part played, cut at its end, with their range in the middle', () => {
    const { rows, bars } = noteBars([note(60, 0, 4), note(62, 6, 4), note(64, 12)], 16, 0, 8)
    expect(rows).toBe(12)
    expect(bars).toEqual([{ start: 0, end: 4, row: 6 }, { start: 6, end: 8, row: 4 }])
  })

  it('cuts notes at the pattern\'s end, and leaves out those that began before the part', () => {
    const notes = [note(60, 0, 4), note(64, 6, 12), note(72, 12, 2)]
    expect(noteBars(notes, 16, 0, 16)).toEqual({
      rows: 13,
      bars: [{ start: 0, end: 4, row: 12 }, { start: 6, end: 16, row: 8 }, { start: 12, end: 14, row: 0 }],
    })
    expect(noteBars(notes, 16, 4, 10).bars).toEqual([{ start: 6, end: 10, row: 5 }])
    expect(noteBars([], 16, 0, 16)).toEqual({ rows: 0, bars: [] })
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
