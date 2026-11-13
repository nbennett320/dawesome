import {
  Pattern,
  isLit,
  litRows,
  patternBars,
  patternLabel,
  patternSeconds,
  stepAt,
  withStep,
  patternHint,
  searchPatterns,
  folderKey,
  folderIdOf,
  folderPathLabel,
  patternFolderInfos,
} from './sequencer'

const pattern = (steps: Record<string, number[]> = {}, length = 16): Pattern =>
  ({ id: 2, name: null, length, steps })

describe('patternLabel', () => {
  it('is the name, or the number from the id', () => {
    expect(patternLabel({ id: 2, name: null })).toBe('Loop 3')
    expect(patternLabel({ id: 2, name: null, kind: 'drum' })).toBe('Loop 3')
    expect(patternLabel({ id: 2, name: null, kind: 'note' })).toBe('Sequence 3')
    expect(patternLabel({ id: 2, name: 'verse' })).toBe('verse')
  })
})

describe('withStep', () => {
  it('lights and clears steps, keeping them in order', () => {
    const lit = withStep(withStep(pattern(), 7, 8, true), 7, 0, true)
    expect(lit.steps).toEqual({ 7: [0, 8] })
    expect(isLit(lit, 7, 8)).toBe(true)

    const cleared = withStep(withStep(lit, 7, 0, false), 7, 8, false)
    // a row with no steps lit has no entry, like the engine
    expect(cleared.steps).toEqual({})
  })

  it("doesn't change the pattern it was given", () => {
    const p = pattern({ 1: [4] })
    withStep(p, 1, 5, true)
    expect(p.steps).toEqual({ 1: [4] })
  })
})

describe('patternBars', () => {
  it('counts bars of the time signature', () => {
    expect(patternBars(pattern({}, 32), 4)).toBe(2)
    expect(patternBars(pattern({}, 24), 3)).toBe(2)
    // set in 4/4, now in 3/4
    expect(patternBars(pattern({}, 16), 3)).toBeNull()
  })
})

describe('patternSeconds', () => {
  it('is the steps as 16th notes at the tempo', () => {
    expect(patternSeconds(pattern({}, 16), 120)).toBe(2)
    expect(patternSeconds(pattern({}, 64), 60)).toBe(16)
  })
})

describe('stepAt', () => {
  it('follows the run, wrapping at the end', () => {
    const p = pattern({}, 16)
    expect(stepAt(0, p, 500)).toBe(0)
    expect(stepAt(130, p, 500)).toBe(1)
    expect(stepAt(2_000, p, 500)).toBe(0)
    expect(stepAt(2_130, p, 500)).toBe(1)
    expect(stepAt(100, p, 0)).toBeNull()
  })
})

describe('litRows', () => {
  it("lists each row's steps in row order, inside the pattern", () => {
    const rows = [5, 3].map(id => ({ id, path: '', label: null, volume: 1, pan: 0, muted: false }))
    expect(litRows(pattern({ 3: [1, 20], 9: [2] }), rows)).toEqual([[], [1]])
  })
})

const named = (id: number, name: string | null, length = 16): Pattern =>
  ({ id, name, length, steps: {} })

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
