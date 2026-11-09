import {
  RecordingFile, describeUse, movedFolderPath, parentFolder, recordingFolders, formatTakeDate, formatTakeDuration, searchTakes, takeUses,
} from './recordings'

describe('formatTakeDuration', () => {
  it('gives minutes and seconds', () => {
    expect(formatTakeDuration(7.4)).toBe('0:07')
    expect(formatTakeDuration(102)).toBe('1:42')
  })

  it('gives hours past an hour', () => {
    expect(formatTakeDuration(3723)).toBe('1:02:03')
  })

  it('shows a dash when the length is unknown', () => {
    expect(formatTakeDuration(null)).toBe('–')
    expect(formatTakeDuration(NaN)).toBe('–')
  })
})

describe('formatTakeDate', () => {
  const now = new Date(2026, 10, 9, 15, 0)

  it('is blank without a time', () => {
    expect(formatTakeDate(null, now)).toBe('')
  })

  it('gives the time for a take from today, and the day otherwise', () => {
    const today = formatTakeDate(new Date(2026, 10, 9, 9, 5).getTime(), now)
    const earlier = formatTakeDate(new Date(2026, 10, 2, 9, 5).getTime(), now)
    const lastYear = formatTakeDate(new Date(2025, 10, 2, 9, 5).getTime(), now)
    expect(today).toMatch(/9.?05/)
    expect(earlier).not.toBe(today)
    expect(lastYear).toMatch(/25/)
  })
})

describe('takeUses', () => {
  it('counts clips and rows by path, skipping pattern clips', () => {
    const uses = takeUses(['/r/a.wav', '/r/a.wav', '', '/r/b.wav'], ['/r/b.wav'])
    expect(uses.get('/r/a.wav')).toEqual({ clips: 2, rows: 0 })
    expect(uses.get('/r/b.wav')).toEqual({ clips: 1, rows: 1 })
    expect(uses.has('')).toBe(false)
  })
})

describe('describeUse', () => {
  it('says where a take is used', () => {
    expect(describeUse(undefined)).toBe('Not used in the project')
    expect(describeUse({ clips: 1, rows: 0 })).toBe('Used by 1 clip in the playlist')
    expect(describeUse({ clips: 2, rows: 1 })).toBe('Used by 2 clips in the playlist and 1 Sequencer row')
  })
})

describe('searchTakes', () => {
  const take = (name: string): RecordingFile => ({ path: `/r/${name}.wav`, name, folder: '', durationSecs: 1, modifiedMs: 0 })
  it('finds takes by name without minding case', () => {
    const takes = [take('song_11092026_0'), take('Vocal idea')]
    expect(searchTakes(takes, 'VOCAL').map(t => t.name)).toEqual(['Vocal idea'])
    expect(searchTakes(takes, ' ')).toEqual(takes)
  })
})

describe('recordingFolders', () => {
  it('keys folders by path, with their parents', () => {
    expect(recordingFolders(['Vocals', 'Vocals/Verse 2'])).toEqual([
      { key: 'Vocals', name: 'Vocals', parent: '' },
      { key: 'Vocals/Verse 2', name: 'Verse 2', parent: 'Vocals' },
    ])
    expect(parentFolder('a/b/c')).toBe('a/b')
    expect(parentFolder('a')).toBe('')
  })
})

describe('movedFolderPath', () => {
  it('moves the folder and what is in it, nothing else', () => {
    expect(movedFolderPath('Vocals', 'Vocals', 'Old/Vocals')).toBe('Old/Vocals')
    expect(movedFolderPath('Vocals/Verse', 'Vocals', 'Old/Vocals')).toBe('Old/Vocals/Verse')
    expect(movedFolderPath('Vocals 2', 'Vocals', 'Old/Vocals')).toBe('Vocals 2')
  })
})
