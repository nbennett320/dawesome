import reducer, {
  chosenPlace, formatOf, MixdownState, setBits, setFileType, setKbps, setMixdownPlace, setWatching,
} from './mixdownSlice'

const initial = (): MixdownState => reducer(undefined, { type: 'init' })

describe('mixdownSlice', () => {
  it('starts as a 16-bit WAV, with the project\'s name and folder', () => {
    const state = initial()
    expect(formatOf(state)).toEqual({ type: 'wav', bits: 'int16' })
    expect(chosenPlace(state, null)).toEqual({ name: null, dir: null })
    expect(state.watching).toBe(false)
  })

  it('sends the format chosen', () => {
    let state = reducer(initial(), setBits('int24'))
    expect(formatOf(state)).toEqual({ type: 'wav', bits: 'int24' })
    state = reducer(reducer(state, setFileType('mp3')), setKbps(192))
    expect(formatOf(state)).toEqual({ type: 'mp3', kbps: 192 })
    // and back, as it was
    expect(formatOf(reducer(state, setFileType('wav')))).toEqual({ type: 'wav', bits: 'int24' })
  })

  it('keeps a chosen name and folder for the project they were chosen in', () => {
    let state = reducer(initial(), setMixdownPlace({ project: '/a/a.dawesome', name: 'take 1' }))
    state = reducer(state, setMixdownPlace({ project: '/a/a.dawesome', dir: '/out' }))
    expect(chosenPlace(state, '/a/a.dawesome')).toEqual({ name: 'take 1', dir: '/out' })
    // another project (or this one, saved somewhere new) has its own
    expect(chosenPlace(state, '/b/b.dawesome')).toEqual({ name: null, dir: null })
    expect(chosenPlace(state, null)).toEqual({ name: null, dir: null })

    state = reducer(state, setMixdownPlace({ project: '/b/b.dawesome', name: 'b' }))
    expect(chosenPlace(state, '/b/b.dawesome')).toEqual({ name: 'b', dir: null })
    expect(chosenPlace(state, '/a/a.dawesome')).toEqual({ name: null, dir: null })
  })

  it('watches a mixdown until it\'s put away', () => {
    const state = reducer(initial(), setWatching(true))
    expect(state.watching).toBe(true)
    expect(reducer(state, setWatching(false)).watching).toBe(false)
  })
})
