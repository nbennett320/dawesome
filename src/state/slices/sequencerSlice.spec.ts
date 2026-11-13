import type { Mock } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { configureStore } from '@reduxjs/toolkit'
import reducer, {
  addSampleRow, addSynthOnTrack, addSynthRow, changeTempo, explodePatternClips, moveRow, selectLastSelectedPattern,
  selectPattern, selectRowKinds,
  replaceRowSynth, selectPianoRollRow, selectSampleRows, selectSynthRows, selectUsedOnly, sequencerSlice,
  setPatternStep, setPianoRollRow, setRowGated, setRowMuted, setRowSamplePitch, setRowStepPitch, setRowVolume,
  setSequencer, setUsedOnly, showPattern,
} from './sequencerSlice'
import { beginGesture, resetGestures } from '../gesture'
import playlistReducer, { paintPatternClips, reduceLoadedProject, reducePainted, setPlaying } from './playlistSlice'
import projectReducer from './projectSlice'
import windowReducer, { selectFocusedView } from './windowSlice'
import mixerReducer from './mixerSlice'
import effectsReducer from './effectsSlice'
import envReducer from './envSlice'
import recordingReducer from './recordingSlice'
import mixdownReducer from './mixdownSlice'
import appearanceReducer from './appearanceSlice'
import pianoRollReducer from './pianoRollSlice'
import { LoadedProject, ProjectView } from '../../types/project'
import { Note, PatternChange, RowChange, SequencerState, UNCONFIRMED } from '../../types/sequencer'
import { freshRev, pattern, sampleRow, sequencerOf, step, steps, synthRow } from '../../types/sequencer.fixture'
import { View } from '../../types/ui'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const note = (pitch: number, start: number, length = 4): Note => ({ pitch, start, length, velocity: 0.8 })

// a sample row (0), then synth rows 3 and 1 (slots 103 and 101), shown in
// that order. patterns 0 (the kick's steps), 2 (row 3's notes), 4 and 6
const sequencer: SequencerState = sequencerOf({
  rows: [sampleRow(0), synthRow(3), synthRow(1)],
  patterns: [
    pattern(0, { notes: { 0: steps(0, 8) } }),
    pattern(2, { name: 'lead', length: 64, notes: { 3: [note(60, 0)] } }),
    pattern(4, { name: 'fill', length: 32, folder: 1 }),
    pattern(6),
  ],
  folders: [{ id: 1, name: 'Fills' }],
})

const init = () => reducer(undefined, { type: 'init' })
const asRoot = (state: ReturnType<typeof init>) => ({ sequencer: state }) as unknown as Parameters<typeof selectSampleRows>[0]

describe('sequencerSlice', () => {
  it('keeps the rows, patterns and folders the engine sends', () => {
    const state = reducer(init(), setSequencer(sequencer))
    expect(state.rows.map(r => r.id)).toEqual([0, 3, 1])
    expect(state.folders).toEqual([{ id: 1, name: 'Fills' }])
    expect(state.patterns[2].folder).toBe(1)
    expect(reducer(state, setSequencer({ ...sequencer, folders: [] })).folders).toEqual([])
  })

  it('selects the first pattern, and keeps the selection while it exists', () => {
    let state = reducer(init(), setSequencer(sequencer))
    expect(state.selected).toBe(0)

    state = reducer(state, selectPattern(4))
    state = reducer(state, setSequencer({ ...sequencer, rows: [] }))
    expect(state.selected).toBe(4)

    // the selected one deleted: back to the first
    state = reducer(state, setSequencer({ ...sequencer, patterns: [sequencer.patterns[1]] }))
    expect(state.selected).toBe(2)

    // and nothing without patterns
    state = reducer(state, setSequencer(sequencerOf()))
    expect(state.selected).toBeNull()
  })

  it("won't select a pattern that doesn't exist", () => {
    const state = reducer(reducer(init(), setSequencer(sequencer)), selectPattern(99))
    expect(state.selected).toBe(0)
  })

  it('remembers the pattern last selected', () => {
    let state = reducer(init(), setSequencer(sequencer))
    // a pattern selected for the project being opened isn't one picked
    expect(state.lastSelected).toBeNull()

    state = reducer(state, selectPattern(6))
    expect(state.lastSelected).toBe(6)
    state = reducer(state, selectPattern(4))
    expect(state.lastSelected).toBe(4)
    expect(selectLastSelectedPattern(asRoot(state))).toBe(4)
    // (one that isn't there isn't)
    state = reducer(state, selectPattern(99))
    expect(state.lastSelected).toBe(4)

    // kept while it's there
    state = reducer(state, setSequencer({ ...sequencer, rows: [] }))
    expect(state.lastSelected).toBe(4)
    // gone with it, and not replaced
    state = reducer(state, setSequencer({ ...sequencer, patterns: sequencer.patterns.filter(p => p.id !== 4) }))
    expect(state.lastSelected).toBeNull()
    expect(state.selected).toBe(0)
  })

  it('forgets the pattern last selected when a project is opened', () => {
    let state = reducer(reducer(init(), setSequencer(sequencer)), selectPattern(2))
    state = reducer(state, reduceLoadedProject({ sequencer, patternClips: [] } as unknown as LoadedProject))
    expect(state.lastSelected).toBeNull()
    expect(state.selected).toBe(0)
  })

  it('selects the pattern clips were painted with', () => {
    let state = reducer(init(), setSequencer(sequencer))
    // a new pattern the paint made
    const painted = {
      clips: [{ id: 40, patternId: 8, trackNumber: 1, startBeat: 0, window: { start: 0, end: 32 } }],
      patternId: 8,
      sequencer: { ...sequencer, patterns: [...sequencer.patterns, pattern(8, { length: 32 })] },
      mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
      trackCount: 4,
    }
    state = reducer(state, reducePainted(painted))
    expect(state.patterns).toHaveLength(5)
    expect(state.selected).toBe(8)
    expect(state.lastSelected).toBe(8)
  })

  it('keeps the row the Piano Roll edits while it’s there, or the first synth row, or the first row', () => {
    let state = reducer(init(), setSequencer(sequencer))
    // the first synth row on screen, whatever the ids
    expect(state.pianoRollRowId).toBe(3)

    state = reducer(state, setPianoRollRow(1))
    // not a row
    state = reducer(state, setPianoRollRow(99))
    expect(state.pianoRollRowId).toBe(1)
    expect(selectPianoRollRow(asRoot(state))?.instrument).toMatchObject({ kind: 'synth', slot: 101 })
    // a sample row
    state = reducer(state, setPianoRollRow(0))
    expect(selectPianoRollRow(asRoot(state))?.instrument).toMatchObject({ kind: 'sample', path: '/drums/0.wav' })

    state = reducer(state, setSequencer({ ...sequencer, rows: [synthRow(3), sampleRow(2)] }))
    expect(state.pianoRollRowId).toBe(3)
    state = reducer(state, setSequencer({ ...sequencer, rows: [sampleRow(2), sampleRow(0)] }))
    expect(state.pianoRollRowId).toBe(2)
    state = reducer(state, setSequencer({ ...sequencer, rows: [] }))
    expect(state.pianoRollRowId).toBeNull()
  })

  it('lists the sample rows and the synth rows apart, each the same list until the rows change', () => {
    const root = asRoot(reducer(init(), setSequencer(sequencer)))
    expect(selectSampleRows(root).map(r => r.id)).toEqual([0])
    expect(selectSynthRows(root).map(r => r.id)).toEqual([3, 1])
    expect(selectSynthRows(root)).toBe(selectSynthRows(root))
  })

  it('gives each row\'s kind, the same map until a row comes or goes', () => {
    const state = reducer(init(), setSequencer(sequencer))
    const kinds = selectRowKinds(asRoot(state))
    expect([...kinds]).toEqual([[0, 'sample'], [3, 'synth'], [1, 'synth']])
    // a knob turned, and the rows moved: the same map
    const turned = reducer(state, setSequencer({ ...sequencer, rows: [synthRow(1), sampleRow(0, { volume: 0.5 }), synthRow(3)] }))
    expect(selectRowKinds(asRoot(turned))).toBe(kinds)
    const added = reducer(state, setSequencer({ ...sequencer, rows: [...sequencer.rows, sampleRow(5)] }))
    expect(selectRowKinds(asRoot(added)).get(5)).toBe('sample')
  })

  it('shows a row’s notes straight away, in order', () => {
    const { reduceNotes } = sequencerSlice.actions
    let state = reducer(init(), setSequencer(sequencer))
    state = reducer(state, reduceNotes({ patternId: 2, rowId: 1, notes: [note(62, 2)], stepsPerBar: 16 }))
    expect(Object.keys(state.patterns[1].notes)).toEqual(['1', '3'])
    state = reducer(state, reduceNotes({ patternId: 2, rowId: 3, notes: [], stepsPerBar: 16 }))
    expect(state.patterns[1].notes).toEqual({ 1: [note(62, 2)] })
    // drawn past the end: the pattern grows to fit, in the same dispatch
    state = reducer(state, reduceNotes({ patternId: 2, rowId: 3, notes: [note(60, 70)], stepsPerBar: 16 }))
    expect(state.patterns[1].length).toBe(80)
  })

  it('shows a step straight away, at the row’s step pitch', () => {
    const { reduceStep } = sequencerSlice.actions
    let state = reducer(init(), setSequencer({ ...sequencer, rows: [sampleRow(0, { stepPitch: 48 })] }))
    state = reducer(state, reduceStep({ patternId: 0, rowId: 0, step: 4, on: true, stepsPerBar: 16 }))
    expect(state.patterns[0].notes).toEqual({ 0: [step(0), step(4, 48), step(8)] })
    // (a row that's gone changes nothing)
    const before = state.patterns
    state = reducer(state, reduceStep({ patternId: 0, rowId: 9, step: 4, on: true, stepsPerBar: 16 }))
    expect(state.patterns).toBe(before)
    // a greyed step past the end: lit, and the pattern a bar longer
    state = reducer(state, reduceStep({ patternId: 0, rowId: 0, step: 20, on: true, stepsPerBar: 16 }))
    expect(state.patterns[0].length).toBe(32)
    expect(state.patterns[0].notes[0]).toContainEqual(step(20, 48))
  })

  it('shows only the rows used in a pattern while asked to, not saved with the project', () => {
    let state = reducer(init(), setUsedOnly(true))
    expect(selectUsedOnly(asRoot(state))).toBe(true)
    state = reducer(state, reduceLoadedProject({ sequencer, patternClips: [] } as unknown as LoadedProject))
    expect(state.usedOnly).toBe(true)
    expect(reducer(state, setUsedOnly(false)).usedOnly).toBe(false)
  })

  it('forgets the run once the engine stops', () => {
    const { setRun } = sequencerSlice.actions
    let state = reducer(init(), setRun({ patternId: 0, startTimeMs: 1, beatIntervalMs: 500, length: 16 }))
    state = reducer(state, setPlaying(true))
    expect(state.run).not.toBeNull()
    state = reducer(state, setPlaying(false))
    expect(state.run).toBeNull()
  })

  it("takes a loaded project's sequencer", () => {
    let state = reducer(init(), setSequencer(sequencerOf({ patterns: [pattern(9)] })))
    state = reducer(state, selectPattern(9))
    state = reducer(state, setPianoRollRow(3))

    state = reducer(state, reduceLoadedProject({ sequencer, patternClips: [] } as unknown as LoadedProject))
    expect(state.patterns).toHaveLength(4)
    expect(state.rows).toHaveLength(3)
    expect(state.selected).toBe(0)
    expect(state.pianoRollRowId).toBe(3)
  })

  it('takes a project with no sequencer', () => {
    const state = reducer(init(), reduceLoadedProject({ sequencer: null, patternClips: [] } as unknown as LoadedProject))
    expect(state.rows).toEqual([])
    expect(state.pianoRollRowId).toBeNull()
  })
})

describe('changeTempo', () => {
  const mockInvoke = invoke as Mock
  // a pattern playing on its own, as the engine reports it after starting
  // it over at the new tempo
  const transport = {
    playing: true, patternId: 4, startTimeMs: 5_000, beatIntervalMs: 666, patternLength: 32,
  }

  it('follows a pattern that started over at the new tempo', async () => {
    mockInvoke.mockImplementation(async (command: string, args?: { val: number }) => {
      if(command === 'set_playlist_tempo') return args?.val
      if(command === 'get_playlist_playing') return true
      if(command === 'get_playlist_transport') return transport
      return undefined
    })
    const actions: { type: string }[] = []

    expect(await changeTempo(90)(a => { actions.push(a as { type: string }); return a })).toBe(true)

    const state = actions.reduce(reducer, init())
    expect(state.run).toEqual({ patternId: 4, startTimeMs: 5_000, beatIntervalMs: 666, length: 32 })
  })

  it('asks nothing more when the tempo was not changed', async () => {
    const dispatch = vi.fn()
    expect(await changeTempo(NaN)(dispatch)).toBe(false)
    expect(mockInvoke).not.toHaveBeenCalled()
  })
})

describe('sequencer edits and undo', () => {
  const mockInvoke = invoke as Mock

  beforeEach(() => {
    mockInvoke.mockReset()
    mockInvoke.mockResolvedValue(sequencer)
    resetGestures()
  })

  const sent = () => mockInvoke.mock.calls.map(([command, args]) => [command, args.gesture])
  const getState = () => ({ sequencer: reducer(init(), setSequencer(sequencer)) })

  it('sends every step of one stroke, and every value of one knob drag, as one gesture', async () => {
    const dispatch = vi.fn()

    // the mouse goes down on a step, and is dragged along the row
    const stroke = beginGesture()
    await setPatternStep(0, 0, 4, true, 16)(dispatch, getState)
    await setPatternStep(0, 0, 5, true, 16)(dispatch, getState)
    await setPatternStep(0, 0, 6, true, 16)(dispatch, getState)
    // the next press: a volume knob
    const drag = beginGesture()
    await setRowVolume(3, 0.8)(dispatch)
    await setRowVolume(3, 0.6)(dispatch)

    expect(stroke).not.toBe(drag)
    expect(sent()).toEqual([
      ['set_pattern_step', stroke], ['set_pattern_step', stroke], ['set_pattern_step', stroke],
      ['set_row_volume', drag], ['set_row_volume', drag],
    ])
    expect(mockInvoke).toHaveBeenCalledWith('set_pattern_step', { patternId: 0, rowId: 0, step: 4, on: true, gesture: stroke, trim: false })
    expect(mockInvoke).toHaveBeenCalledWith('set_row_volume', { rowId: 3, volume: 0.6, gesture: drag })
  })

  it('sends a step toggled from the keyboard, and a mute, as steps of their own', async () => {
    const dispatch = vi.fn()
    await setPatternStep(0, 0, 4, true, 16)(dispatch, getState)
    // a click is a press too, but a mute is always its own step
    beginGesture()
    await setRowMuted(1, true)(dispatch)

    expect(sent()).toEqual([['set_pattern_step', null], ['set_row_muted', null]])
    expect(mockInvoke).toHaveBeenCalledWith('set_row_muted', { rowId: 1, muted: true, gesture: null })
  })
})

describe('adding rows', () => {
  const mockInvoke = invoke as Mock

  const makeStore = () => configureStore({
    reducer: {
      sequencer: reducer,
      playlist: playlistReducer,
      project: projectReducer,
      window: windowReducer,
      mixer: mixerReducer,
      effects: effectsReducer,
      env: envReducer,
      recording: recordingReducer,
      mixdown: mixdownReducer,
      appearance: appearanceReducer,
      pianoRoll: pianoRollReducer,
    },
  })

  beforeEach(() => {
    mockInvoke.mockReset()
    resetGestures()
  })

  const withSynth = sequencerOf({ rows: [synthRow(4)] })

  it('adds a dropped instrument as a synth row, bringing a pattern along with the first', async () => {
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'add_synth_row') return { rowId: 4, sequencer: withSynth }
      if(command === 'add_pattern') return { id: 6, sequencer: { ...withSynth, patterns: [pattern(6)] } }
      return undefined
    })
    const store = makeStore()

    expect(await store.dispatch(addSynthRow('vst3:4'))).toBe(4)
    expect(mockInvoke.mock.calls.map(([command, args]) => [command, args])).toEqual([
      ['add_synth_row', { pluginId: 'vst3:4' }],
      ['add_pattern', { bars: 1 }],
    ])
    expect(store.getState().sequencer.selected).toBe(6)
  })

  it('adds a dropped sample as a sample row, with a pattern only when there’s none', async () => {
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'add_sample_row') return { rowId: 0, sequencer }
      return undefined
    })
    const store = makeStore()

    await store.dispatch(addSampleRow('/drums/kick.wav'))
    expect(mockInvoke.mock.calls).toEqual([['add_sample_row', { path: '/drums/kick.wav' }]])
    expect(store.getState().sequencer.rows).toHaveLength(3)
  })

  it('says why an instrument couldn’t be added', async () => {
    mockInvoke.mockRejectedValue(new Error('not an instrument'))
    const store = makeStore()
    expect(await store.dispatch(addSynthRow('vst3:x'))).toBeNull()
    expect(store.getState().project.notice).toEqual({ kind: 'error', message: 'Couldn’t add the instrument: not an instrument' })
  })

  it('replaces a synth row’s plugin, keeping the row, or says why it couldn’t', async () => {
    const replaced = synthRow(3, { slot: 140, name: 'Keys' })
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'replace_row_synth') return { row: replaced }
      return undefined
    })
    const store = makeStore()
    store.dispatch(setSequencer(sequencer))
    await store.dispatch(replaceRowSynth(3, 'vst3:keys'))
    expect(mockInvoke).toHaveBeenCalledWith('replace_row_synth', { rowId: 3, pluginId: 'vst3:keys' })
    expect(store.getState().sequencer.rows.map(r => r.id)).toEqual([0, 3, 1])
    expect(store.getState().sequencer.rows[1]).toBe(replaced)

    mockInvoke.mockRejectedValue('Keys couldn’t be loaded')
    await store.dispatch(replaceRowSynth(3, 'vst3:keys'))
    expect(store.getState().project.notice).toEqual({ kind: 'error', message: 'Couldn’t replace the instrument: Keys couldn’t be loaded' })
  })

  it('sets a row’s step pitch (the patterns its notes move in coming back too), sample pitch and gate', async () => {
    const moved = { ...sequencer, rows: [sampleRow(0, { stepPitch: 48 }), ...sequencer.rows.slice(1)] }
    const gated = sampleRow(0, { stepPitch: 48, samplePitch: 72, gated: true })
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'set_row_step_pitch') return moved
      if(command === 'set_row_sample_pitch' || command === 'set_row_gated') return { row: gated }
      return undefined
    })
    const store = makeStore()
    store.dispatch(setSequencer(sequencer))
    await store.dispatch(setRowStepPitch(0, 48))
    expect(store.getState().sequencer.rows[0].stepPitch).toBe(48)
    await store.dispatch(setRowSamplePitch(0, 72))
    await store.dispatch(setRowGated(0, true))
    expect(store.getState().sequencer.rows[0]).toBe(gated)
    expect(mockInvoke.mock.calls.map(([command, args]) => [command, args])).toEqual([
      ['set_row_step_pitch', { rowId: 0, pitch: 48 }],
      ['set_row_sample_pitch', { rowId: 0, pitch: 72 }],
      ['set_row_gated', { rowId: 0, gated: true }],
    ])
  })

  it('paints clips into the playlist, the sequencer and the mixer', async () => {
    const mixer = {
      tracks: [{ trackNumber: 4, volumeDb: -3, pan: 0, mute: false, solo: false }],
      master: { volumeDb: 0, pan: 0, mute: false, solo: false },
    }
    mockInvoke.mockImplementation(async (command: string) => {
      if(command !== 'paint_pattern_clips') return undefined
      return {
        clips: [
          { id: 40, patternId: 6, trackNumber: 4, startBeat: 2, window: null },
          { id: 41, patternId: 6, trackNumber: 4, startBeat: 6, window: { start: 0, end: 8 } },
        ],
        patternId: 6,
        sequencer,
        mixer,
        trackCount: 5,
      }
    })
    const store = makeStore()

    const painted = await store.dispatch(paintPatternClips(6, 4, 2, 8))
    expect(painted?.clips).toHaveLength(2)
    expect(mockInvoke).toHaveBeenCalledWith(
      'paint_pattern_clips', { patternId: 6, trackNumber: 4, startBeat: 2, endBeat: 8 })
    const state = store.getState()
    expect(state.playlist.playlistItems.map(i => [i.id, i.patternId, i.trackNumber, i.startBeat])).toEqual([
      [40, 6, 4, 2], [41, 6, 4, 6],
    ])
    expect(state.playlist.clipWindows).toEqual({ 41: { start: 0, end: 8 } })
    expect(state.playlist.ui.trackCount).toBe(5)
    expect(state.sequencer.selected).toBe(6)
    expect(state.sequencer.lastSelected).toBe(6)
    expect(state.mixer.tracks[4]).toMatchObject({ volumeDb: -3 })
  })

  it('explodes a pattern clip into a clip per row, each looping as it did, on tracks named after them', async () => {
    const looping = { start: 4, end: 40 }
    // a clip per row: row 0's on the clip's track, row 3's below it
    const exploded = {
      removed: [30],
      clips: [
        { id: 31, patternId: 7, trackNumber: 1, startBeat: 2, window: looping },
        { id: 32, patternId: 8, trackNumber: 2, startBeat: 2, window: looping },
      ],
      sequencer: sequencerOf({
        rows: sequencer.rows,
        patterns: [...sequencer.patterns, pattern(7, { notes: { 0: steps(0, 8) } }), pattern(8, { notes: { 3: [note(60, 0)] } })],
        folders: sequencer.folders,
      }),
      mixer: {
        tracks: [{ trackNumber: 2, volumeDb: 0, pan: 0, mute: false, solo: false, name: 'Synth 3' }],
        master: { volumeDb: 0, pan: 0, mute: false, solo: false },
      },
    }
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'explode_pattern_clips') return exploded
      if(command === 'get_playlist_data') return { beats: 0, trackCount: 3 }
      return undefined
    })
    const store = makeStore()
    store.dispatch(reducePainted({
      clips: [{ id: 30, patternId: 0, trackNumber: 1, startBeat: 2, window: looping }],
      patternId: 0, sequencer, mixer: exploded.mixer, trackCount: 2,
    }))

    await store.dispatch(explodePatternClips([30]))
    expect(mockInvoke).toHaveBeenCalledWith('explode_pattern_clips', { ids: [30] })
    const state = store.getState()
    expect(state.playlist.playlistItems.map(i => [i.id, i.patternId, i.trackNumber, i.startBeat])).toEqual([
      [31, 7, 1, 2], [32, 8, 2, 2],
    ])
    expect(state.playlist.clipWindows).toEqual({ 31: looping, 32: looping })
    expect(state.sequencer.patterns.map(p => p.id)).toEqual([0, 2, 4, 6, 7, 8])
    expect(state.mixer.names[2]).toBe('Synth 3')
    expect(state.playlist.ui.trackCount).toBe(3)
  })

  it('leaves the playlist as it is when the engine won’t explode', async () => {
    mockInvoke.mockRejectedValue('no pattern clip with id 30')
    vi.spyOn(console, 'error').mockImplementation(() => {})
    const store = makeStore()
    const before = store.getState().playlist
    await store.dispatch(explodePatternClips([30]))
    expect(store.getState().playlist).toBe(before)
    // and asks nothing for no clips
    mockInvoke.mockClear()
    await store.dispatch(explodePatternClips([]))
    expect(mockInvoke).not.toHaveBeenCalled()
  })

  it('paints nothing when the engine refuses', async () => {
    mockInvoke.mockRejectedValue('no pattern with id 99')
    const store = makeStore()
    vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(await store.dispatch(paintPatternClips(99, 0, 0, 4))).toBeNull()
    expect(store.getState().playlist.playlistItems).toEqual([])
  })

  it('starts a synth row on a track at the playhead, and opens the Piano Roll on its pattern', async () => {
    const project: ProjectView = {
      tempo: 120,
      clips: [],
      patternClips: [{ id: 30, patternId: 6, trackNumber: 2, startBeat: 8, window: { start: 0, end: 16 } }],
      mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
      sequencer: { ...sequencer, rows: [...sequencer.rows, synthRow(12)] },
    }
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'get_playlist_transport') return { positionMs: 4000, beatIntervalMs: 500 }
      if(command === 'add_synth_on_track') return { rowId: 12, patternId: 6, project }
      return undefined
    })
    const store = makeStore()

    await store.dispatch(addSynthOnTrack('vst3:12', 2))
    expect(mockInvoke).toHaveBeenCalledWith('add_synth_on_track', { pluginId: 'vst3:12', trackNumber: 2, dropBeat: 8 })
    const state = store.getState()
    expect(state.sequencer.selected).toBe(6)
    expect(state.sequencer.pianoRollRowId).toBe(12)
    expect(state.playlist.playlistItems.map(item => item.patternId)).toEqual([6])
    expect(selectFocusedView(state)).toBe(View.PianoRoll)
  })

  it('opens a pattern in the Pattern Sequencer, whatever it holds', () => {
    const store = makeStore()
    store.dispatch(setSequencer(sequencer))
    store.dispatch(showPattern(2))
    expect(selectFocusedView(store.getState())).toBe(View.PatternSequencer)
    expect(store.getState().sequencer.selected).toBe(2)
    store.dispatch(showPattern(0))
    expect(store.getState().sequencer.selected).toBe(0)
    // (a pattern that's gone: nothing)
    store.dispatch(showPattern(99))
    expect(store.getState().sequencer.selected).toBe(0)
  })
})

describe('moving a row', () => {
  const mockInvoke = invoke as Mock
  const three: SequencerState = { ...sequencer, rows: [sampleRow(0), synthRow(1), sampleRow(2)] }
  const order = (store: { getState: () => { sequencer: { rows: Array<{ id: number }> } } }) =>
    store.getState().sequencer.rows.map(r => r.id)

  const makeStore = () => {
    const store = configureStore({ reducer: { sequencer: reducer } })
    store.dispatch(setSequencer(three))
    return store
  }

  it('shows the move straight away, then as the engine has it', async () => {
    let answer: (s: SequencerState) => void = () => {}
    mockInvoke.mockReset()
    mockInvoke.mockImplementation(() => new Promise(resolve => { answer = resolve }))
    const store = makeStore()

    const moved = store.dispatch(moveRow(2, 0))
    expect(order(store)).toEqual([2, 0, 1])
    expect(mockInvoke).toHaveBeenCalledWith('move_row', { rowId: 2, index: 0 })

    answer({ ...three, rows: [sampleRow(2), sampleRow(0), synthRow(1)] })
    await moved
    expect(order(store)).toEqual([2, 0, 1])
  })

  it('goes back to the engine\'s rows when the move fails', async () => {
    mockInvoke.mockReset()
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'move_row') throw new Error('no row with id 0')
      return three
    })
    vi.spyOn(console, 'error').mockImplementation(() => {})
    const store = makeStore()

    await store.dispatch(moveRow(0, 2))
    expect(order(store)).toEqual([0, 1, 2])
    expect(mockInvoke).toHaveBeenCalledWith('get_sequencer', {})
  })
})

describe('what the engine answers', () => {
  const mockInvoke = invoke as Mock
  // the kick (0) and Pad (3) both have notes in pattern 0
  const engine: SequencerState = sequencerOf({
    rows: [sampleRow(0), synthRow(3, { name: 'Pad' }), sampleRow(5)],
    patterns: [pattern(0, { notes: { 0: steps(0, 8), 3: [note(60, 0)] } }), pattern(2, { length: 64 })],
    folders: [{ id: 1, name: 'Fills' }, { id: 4, name: 'Keys', parent: 1 }],
  })
  // the engine's answer as it arrives: new objects, whatever it holds
  const sent = <T>(answer: T): T => JSON.parse(JSON.stringify(answer))

  const makeStore = () => {
    const store = configureStore({ reducer: { sequencer: reducer } })
    store.dispatch(setSequencer(sent(engine)))
    return store
  }

  beforeEach(() => {
    mockInvoke.mockReset()
    resetGestures()
  })

  it('keeps all it has when the whole sequencer comes back unchanged', () => {
    const state = reducer(init(), setSequencer(sent(engine)))
    expect(reducer(state, setSequencer(sent(engine)))).toBe(state)
  })

  it('takes only the rows, patterns and folders the whole sequencer changed', () => {
    const state = reducer(init(), setSequencer(sent(engine)))
    const changed = sent(engine)
    changed.patterns[1] = { ...changed.patterns[1], rev: freshRev(), name: 'chorus' }
    changed.rows[2] = { ...changed.rows[2], rev: freshRev(), muted: true }

    const next = reducer(state, setSequencer(changed))
    expect(next.patterns[0]).toBe(state.patterns[0])
    expect(next.patterns[1].name).toBe('chorus')
    expect(next.rows.slice(0, 2)).toEqual(state.rows.slice(0, 2))
    next.rows.slice(0, 2).forEach((row, i) => expect(row).toBe(state.rows[i]))
    expect(next.rows[2].muted).toBe(true)
    expect(next.folders).toBe(state.folders)

    // a folder renamed is a new list of folders
    const renamed = sent(changed)
    renamed.folders[1].name = 'Pads'
    expect(reducer(next, setSequencer(renamed)).folders).toEqual(renamed.folders)
  })

  it('replaces just the one pattern a step changed, keeping the other rows’ notes', async () => {
    const store = makeStore()
    const before = store.getState().sequencer
    const lit: PatternChange = sent({ pattern: { ...engine.patterns[0], rev: freshRev(), notes: { ...engine.patterns[0].notes, 0: steps(0, 4, 8) } } })
    mockInvoke.mockResolvedValue(lit)

    await store.dispatch(setPatternStep(0, 0, 4, true, 16))
    const after = store.getState().sequencer
    expect(after.patterns[0]).toEqual(lit.pattern)
    expect(after.patterns[1]).toBe(before.patterns[1])
    expect(after.rows).toBe(before.rows)
    expect(after.folders).toBe(before.folders)
    // Pad's notes didn't change: still the list it had
    expect(after.patterns[0].notes[3]).toBe(before.patterns[0].notes[3])
  })

  it('replaces just the row a knob changed', async () => {
    const store = makeStore()
    const before = store.getState().sequencer
    const sampleRows = selectSampleRows(asRoot(before))
    const turned: RowChange = sent({ row: { ...engine.rows[1], rev: freshRev(), volume: 0.5 } })
    mockInvoke.mockResolvedValue(turned)

    await store.dispatch(setRowVolume(3, 0.5))
    const after = store.getState().sequencer
    expect(after.rows[1].volume).toBe(0.5)
    expect(after.rows[0]).toBe(before.rows[0])
    expect(after.rows[2]).toBe(before.rows[2])
    expect(after.patterns).toBe(before.patterns)
    // a synth row's knob leaves the sample rows' list as it was
    expect(selectSampleRows(asRoot(after))).toBe(sampleRows)
    expect(selectSynthRows(asRoot(after))[0].volume).toBe(0.5)
  })

  it('always replaces what it showed before the engine answered', async () => {
    const store = makeStore()
    let answer: (change: PatternChange) => void = () => {}
    mockInvoke.mockImplementation(() => new Promise(resolve => { answer = resolve }))

    const stepped = store.dispatch(setPatternStep(0, 0, 4, true, 16))
    const shown = store.getState().sequencer.patterns[0]
    expect(shown.rev).toBe(UNCONFIRMED)

    // the engine made the same of it
    const confirmed = sent({ ...shown, rev: freshRev() })
    answer({ pattern: confirmed })
    await stepped
    const now = store.getState().sequencer.patterns[0]
    expect(now).not.toBe(shown)
    expect(now.rev).toBe(confirmed.rev)
    expect(now.notes[0]).toBe(shown.notes[0])
  })

  it('goes back to the engine’s pattern when a step is refused', async () => {
    const store = makeStore()
    const before = store.getState().sequencer.patterns[0]
    vi.spyOn(console, 'error').mockImplementation(() => {})
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'set_pattern_step') throw new Error('no row with id 0')
      return sent(engine)
    })

    await store.dispatch(setPatternStep(0, 0, 4, true, 16))
    expect(mockInvoke).toHaveBeenCalledWith('get_sequencer', {})
    const now = store.getState().sequencer.patterns[0]
    expect(now.rev).toBe(before.rev)
    expect(now.notes).toEqual(before.notes)
  })

  it('ignores an answer about a pattern or row that’s gone', () => {
    const { reducePattern, reduceRow } = sequencerSlice.actions
    const state = reducer(init(), setSequencer(sent(engine)))
    expect(reducer(state, reducePattern({ pattern: pattern(9) }))).toBe(state)
    expect(reducer(state, reduceRow({ row: sampleRow(9) }))).toBe(state)
    // nor one that changes nothing
    expect(reducer(state, reducePattern(sent({ pattern: engine.patterns[1] })))).toBe(state)
  })
})
