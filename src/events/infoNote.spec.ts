import { onInfoNote, postInfoNote } from './infoNote'

describe('info notes', () => {
  it('reach whoever is listening, until they stop', () => {
    const heard: string[] = []
    const stop = onInfoNote(text => heard.push(text))
    postInfoNote('Loop start · bar 9')
    stop()
    postInfoNote('Song end · bar 17')
    expect(heard).toEqual(['Loop start · bar 9'])
  })
})
