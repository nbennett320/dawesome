import type { Mock } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { configureStore } from '@reduxjs/toolkit'
import reducer, {
  addSynth, addSynthOnTrack, changeTempo, selectFoldersOf, selectLastSelectedPattern, selectPatternsOf, sequencerSlice, selectPattern,
  setPatternStep, setPianoRollSynth, setSequencer, setSequencerRowMuted, setSequencerRowVolume, setSynthVolume,
  showPattern,
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
import { SequencerState, Synth } from '../../types/sequencer'
import { View } from '../../types/ui'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

const synth = (id: number, name = `Synth ${id}`): Synth => ({
  id, pluginId: `vst3:${id}`, name, vendor: 'Acme', volume: 1, muted: false, missing: false, hasEditor: true,
})

const sequencer: SequencerState = {
  rows: [{ id: 0, path: '/drums/kick.wav', label: null, volume: 1, pan: 0, muted: false }],
  synths: [],
  patterns: [
    { id: 0, name: null, length: 16, steps: { 0: [0, 8] } },
    { id: 4, name: 'fill', length: 32, steps: {}, folder: 1 },
  ],
  folders: [{ id: 1, name: 'Fills' }],
}

// drum patterns 0 and 4, note patterns 2 and 6, and two synths
const both: SequencerState = {
  ...sequencer,
  synths: [synth(10), synth(11)],
  patterns: [
    sequencer.patterns[0],
    { id: 2, name: 'lead', kind: 'note', length: 64, steps: {}, notes: { 10: [{ pitch: 60, start: 0, length: 4, velocity: 0.8 }] } },
    { ...sequencer.patterns[1], kind: 'drum' },
    { id: 6, name: null, kind: 'note', length: 16, steps: {}, notes: {} },
  ],
  folders: [{ id: 1, name: 'Fills', kind: 'drum' }, { id: 3, name: 'Leads', kind: 'note' }],
}

const init = () => reducer(undefined, { type: 'init' })

describe('sequencerSlice', () => {
  it('keeps the pattern folders the engine sends', () => {
    const state = reducer(init(), setSequencer(sequencer))
    expect(state.folders).toEqual([{ id: 1, name: 'Fills' }])
    expect(state.patterns[1].folder).toBe(1)
    expect(reducer(state, setSequencer({ ...sequencer, folders: [] })).folders).toEqual([])
  })

  it('selects the first pattern, and keeps the selection while it exists', () => {
    let state = reducer(init(), setSequencer(sequencer))
    expect(state.selected.drum).toBe(0)

    state = reducer(state, selectPattern(4))
    state = reducer(state, setSequencer({ ...sequencer, rows: [] }))
    expect(state.selected.drum).toBe(4)

    // the selected one deleted: back to the first
    state = reducer(state, setSequencer({ ...sequencer, patterns: [sequencer.patterns[0]] }))
    expect(state.selected.drum).toBe(0)

    // and nothing without patterns
    state = reducer(state, setSequencer({ rows: [], synths: [], patterns: [], folders: [] }))
    expect(state.selected.drum).toBeNull()
  })

  it("won't select a pattern that doesn't exist", () => {
    const state = reducer(reducer(init(), setSequencer(sequencer)), selectPattern(99))
    expect(state.selected.drum).toBe(0)
  })

  it('keeps a selected pattern of each kind, a pattern without a kind being a drum pattern', () => {
    let state = reducer(init(), setSequencer(both))
    expect(state.selected).toEqual({ drum: 0, note: 2 })

    // picking one changes only its own kind's
    state = reducer(state, selectPattern(6))
    state = reducer(state, selectPattern(4))
    expect(state.selected).toEqual({ drum: 4, note: 6 })

    // a note pattern deleted: the first note pattern, not a drum one
    state = reducer(state, setSequencer({ ...both, patterns: both.patterns.filter(p => p.id !== 6) }))
    expect(state.selected).toEqual({ drum: 4, note: 2 })
    state = reducer(state, setSequencer(sequencer))
    expect(state.selected).toEqual({ drum: 4, note: null })
  })

  it('remembers the pattern last selected, of either kind', () => {
    let state = reducer(init(), setSequencer(both))
    // a pattern selected for the project being opened isn't one picked
    expect(state.lastSelected).toBeNull()

    state = reducer(state, selectPattern(6))
    expect(state.lastSelected).toBe(6)
    state = reducer(state, selectPattern(4))
    expect(state.lastSelected).toBe(4)
    expect(selectLastSelectedPattern({ sequencer: state } as unknown as Parameters<typeof selectLastSelectedPattern>[0])).toBe(4)
    // (one that isn't there isn't)
    state = reducer(state, selectPattern(99))
    expect(state.lastSelected).toBe(4)

    // kept while it's there
    state = reducer(state, setSequencer({ ...both, rows: [] }))
    expect(state.lastSelected).toBe(4)
    // gone with it, and not replaced: the other kind's selection stays its own
    state = reducer(state, setSequencer({ ...both, patterns: both.patterns.filter(p => p.id !== 4) }))
    expect(state.lastSelected).toBeNull()
    expect(state.selected).toEqual({ drum: 0, note: 6 })
  })

  it('forgets the pattern last selected when a project is opened', () => {
    let state = reducer(reducer(init(), setSequencer(both)), selectPattern(2))
    state = reducer(state, reduceLoadedProject({ sequencer: both, patternClips: [] } as unknown as LoadedProject))
    expect(state.lastSelected).toBeNull()
  })

  it('selects the pattern clips were painted with', () => {
    let state = reducer(init(), setSequencer(both))
    // a new note pattern the paint made
    const painted = {
      clips: [{ id: 40, patternId: 8, trackNumber: 1, startBeat: 0, window: null }],
      patternId: 8,
      sequencer: { ...both, patterns: [...both.patterns, { id: 8, name: null, kind: 'note' as const, length: 32, steps: {}, notes: {} }] },
      mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
      trackCount: 4,
    }
    state = reducer(state, reducePainted(painted))
    expect(state.patterns).toHaveLength(5)
    expect(state.selected).toEqual({ drum: 0, note: 8 })
    expect(state.lastSelected).toBe(8)
  })

  it('keeps the synth the Piano Roll edits while it’s there, or the first', () => {
    let state = reducer(init(), setSequencer(both))
    expect(state.synths.map(s => s.id)).toEqual([10, 11])
    expect(state.pianoRollSynthId).toBe(10)

    state = reducer(state, setPianoRollSynth(11))
    state = reducer(state, setPianoRollSynth(99))
    expect(state.pianoRollSynthId).toBe(11)

    state = reducer(state, setSequencer({ ...both, synths: [synth(10)] }))
    expect(state.pianoRollSynthId).toBe(10)
    state = reducer(state, setSequencer(sequencer))
    expect(state.pianoRollSynthId).toBeNull()
  })

  it('lists each kind’s patterns and folders apart', () => {
    const state = { sequencer: reducer(init(), setSequencer(both)) } as never
    expect(selectPatternsOf('drum')(state).map(p => p.id)).toEqual([0, 4])
    expect(selectPatternsOf('note')(state).map(p => p.id)).toEqual([2, 6])
    expect(selectFoldersOf('drum')(state).map(f => f.name)).toEqual(['Fills'])
    expect(selectFoldersOf('note')(state).map(f => f.name)).toEqual(['Leads'])
    // the same list until the patterns change
    expect(selectPatternsOf('note')(state)).toBe(selectPatternsOf('note')(state))
  })

  it('shows a synth’s notes straight away', () => {
    const { reduceNotes } = sequencerSlice.actions
    let state = reducer(init(), setSequencer(both))
    const note = { pitch: 62, start: 2, length: 1, velocity: 0.8 }
    state = reducer(state, reduceNotes({ patternId: 2, synthId: 11, notes: [note] }))
    expect(Object.keys(state.patterns[1].notes ?? {})).toEqual(['10', '11'])
    state = reducer(state, reduceNotes({ patternId: 2, synthId: 10, notes: [] }))
    expect(state.patterns[1].notes).toEqual({ 11: [note] })
  })

  it('shows a step straight away', () => {
    const { reduceStep } = sequencerSlice.actions
    let state = reducer(init(), setSequencer(sequencer))
    state = reducer(state, reduceStep({ patternId: 0, rowId: 0, step: 4, on: true }))
    expect(state.patterns[0].steps).toEqual({ 0: [0, 4, 8] })
  })

  it('forgets the run once the engine stops', () => {
    const { setRun } = sequencerSlice.actions
    let state = reducer(init(), setRun({ patternId: 0, startTimeMs: 1, beatIntervalMs: 500 }))
    state = reducer(state, setPlaying(true))
    expect(state.run).not.toBeNull()
    state = reducer(state, setPlaying(false))
    expect(state.run).toBeNull()
  })

  it("takes a loaded project's sequencer", () => {
    let state = reducer(init(), setSequencer({ rows: [], synths: [], patterns: [{ id: 9, name: null, length: 16, steps: {} }], folders: [] }))
    state = reducer(state, selectPattern(9))

    state = reducer(state, reduceLoadedProject({ sequencer: both, patternClips: [] } as unknown as LoadedProject))
    expect(state.patterns).toHaveLength(4)
    expect(state.rows).toHaveLength(1)
    expect(state.synths).toHaveLength(2)
    expect(state.selected).toEqual({ drum: 0, note: 2 })
  })

  it('takes a project from an older engine, without synths', () => {
    const state = reducer(init(), reduceLoadedProject({ sequencer, patternClips: [] } as unknown as LoadedProject))
    expect(state.synths).toEqual([])
    expect(state.pianoRollSynthId).toBeNull()
  })
})

describe('changeTempo', () => {
  const mockInvoke = invoke as Mock
  // a pattern playing on its own, as the engine reports it after starting
  // it over at the new tempo
  const transport = {
    playing: true, patternId: 4, startTimeMs: 5_000, beatIntervalMs: 666,
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
    expect(state.run).toEqual({ patternId: 4, startTimeMs: 5_000, beatIntervalMs: 666 })
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

  it('sends every step of one stroke, and every value of one knob drag, as one gesture', async () => {
    const dispatch = vi.fn()

    // the mouse goes down on a step, and is dragged along the row
    const stroke = beginGesture()
    await setPatternStep(0, 0, 4, true)(dispatch)
    await setPatternStep(0, 0, 5, true)(dispatch)
    await setPatternStep(0, 0, 6, true)(dispatch)
    // the next press: a volume knob
    const drag = beginGesture()
    await setSequencerRowVolume(0, 0.8)(dispatch)
    await setSequencerRowVolume(0, 0.6)(dispatch)

    expect(stroke).not.toBe(drag)
    expect(sent()).toEqual([
      ['set_pattern_step', stroke], ['set_pattern_step', stroke], ['set_pattern_step', stroke],
      ['set_sequencer_row_volume', drag], ['set_sequencer_row_volume', drag],
    ])
  })

  it('sends a step toggled from the keyboard, and a mute, as steps of their own', async () => {
    const dispatch = vi.fn()
    await setPatternStep(0, 0, 4, true)(dispatch)
    // a click is a press too, but a mute is always its own step
    beginGesture()
    await setSequencerRowMuted(0, true)(dispatch)

    expect(sent()).toEqual([['set_pattern_step', null], ['set_sequencer_row_muted', undefined]])
  })
})

describe('synths', () => {
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

  it('adds a dropped instrument, bringing a note pattern along with the first', async () => {
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'add_synth') return { synthId: 10, sequencer: { ...sequencer, synths: [synth(10)] } }
      if(command === 'add_pattern') {
        return { id: 6, sequencer: { ...sequencer, synths: [synth(10)], patterns: [...sequencer.patterns, both.patterns[3]] } }
      }
      return undefined
    })
    const store = makeStore()

    expect(await store.dispatch(addSynth('vst3:10'))).toBe(10)
    expect(mockInvoke.mock.calls.map(([command, args]) => [command, args])).toEqual([
      ['add_synth', { pluginId: 'vst3:10' }],
      ['add_pattern', { bars: 1, kind: 'note' }],
    ])
    expect(store.getState().sequencer.selected.note).toBe(6)
  })

  it('says why an instrument couldn’t be added', async () => {
    mockInvoke.mockRejectedValue(new Error('not an instrument'))
    const store = makeStore()
    expect(await store.dispatch(addSynth('vst3:x'))).toBeNull()
    expect(store.getState().project.notice).toEqual({ kind: 'error', message: 'Couldn’t add the instrument: not an instrument' })
  })

  it('sends one knob drag as one gesture', async () => {
    mockInvoke.mockResolvedValue(both)
    const drag = beginGesture()
    await setSynthVolume(10, 0.5)(vi.fn())
    expect(mockInvoke).toHaveBeenCalledWith('set_synth_volume', { synthId: 10, volume: 0.5, gesture: drag })
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
        sequencer: both,
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
    expect(state.sequencer.selected.note).toBe(6)
    expect(state.sequencer.lastSelected).toBe(6)
    expect(state.mixer.tracks[4]).toMatchObject({ volumeDb: -3 })
  })

  it('paints nothing when the engine refuses', async () => {
    mockInvoke.mockRejectedValue('no pattern with id 99')
    const store = makeStore()
    vi.spyOn(console, 'error').mockImplementation(() => {})
    expect(await store.dispatch(paintPatternClips(99, 0, 0, 4))).toBeNull()
    expect(store.getState().playlist.playlistItems).toEqual([])
  })

  it('starts a synth on a track at the playhead, and opens the Piano Roll on its pattern', async () => {
    const project: ProjectView = {
      tempo: 120,
      clips: [],
      patternClips: [{ id: 30, patternId: 6, trackNumber: 2, startBeat: 8, window: null }],
      mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
      sequencer: { ...both, synths: [synth(10), synth(11), synth(12)] },
    }
    mockInvoke.mockImplementation(async (command: string) => {
      if(command === 'get_playlist_transport') return { positionMs: 4000, beatIntervalMs: 500 }
      if(command === 'add_synth_on_track') return { synthId: 12, patternId: 6, project }
      return undefined
    })
    const store = makeStore()

    await store.dispatch(addSynthOnTrack('vst3:12', 2))
    expect(mockInvoke).toHaveBeenCalledWith('add_synth_on_track', { pluginId: 'vst3:12', trackNumber: 2, dropBeat: 8 })
    const state = store.getState()
    expect(state.sequencer.selected.note).toBe(6)
    expect(state.sequencer.pianoRollSynthId).toBe(12)
    expect(state.playlist.playlistItems.map(item => item.patternId)).toEqual([6])
    expect(selectFocusedView(state)).toBe(View.PianoRoll)
  })

  it('opens a pattern in the sequencer of its kind', () => {
    const store = makeStore()
    store.dispatch(setSequencer(both))
    store.dispatch(showPattern(6))
    expect(selectFocusedView(store.getState())).toBe(View.NoteSequencer)
    expect(store.getState().sequencer.selected.note).toBe(6)
    store.dispatch(showPattern(4))
    expect(selectFocusedView(store.getState())).toBe(View.Sequencer)
  })
})
