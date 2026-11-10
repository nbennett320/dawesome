import { invoke } from '@tauri-apps/api/core'
import reducer, {
  changeTempo, sequencerSlice, selectPattern, setPatternStep, setSequencer, setSequencerRowMuted,
  setSequencerRowVolume,
} from './sequencerSlice'
import { beginGesture, resetGestures } from '../gesture'
import { reduceLoadedProject, setPlaying } from './playlistSlice'
import { LoadedProject } from '../../types/project'
import { SequencerState } from '../../types/sequencer'

jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))

const sequencer: SequencerState = {
  rows: [{ id: 0, path: '/drums/kick.wav', volume: 1, muted: false }],
  patterns: [
    { id: 0, name: null, length: 16, steps: { 0: [0, 8] } },
    { id: 4, name: 'fill', length: 32, steps: {}, folder: 1 },
  ],
  folders: [{ id: 1, name: 'Fills' }],
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
    expect(state.selectedPatternId).toBe(0)

    state = reducer(state, selectPattern(4))
    state = reducer(state, setSequencer({ ...sequencer, rows: [] }))
    expect(state.selectedPatternId).toBe(4)

    // the selected one deleted: back to the first
    state = reducer(state, setSequencer({ ...sequencer, patterns: [sequencer.patterns[0]] }))
    expect(state.selectedPatternId).toBe(0)

    // and nothing without patterns
    state = reducer(state, setSequencer({ rows: [], patterns: [], folders: [] }))
    expect(state.selectedPatternId).toBeNull()
  })

  it("won't select a pattern that doesn't exist", () => {
    const state = reducer(reducer(init(), setSequencer(sequencer)), selectPattern(99))
    expect(state.selectedPatternId).toBe(0)
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
    let state = reducer(init(), setSequencer({ rows: [], patterns: [{ id: 9, name: null, length: 16, steps: {} }], folders: [] }))
    state = reducer(state, selectPattern(9))

    state = reducer(state, reduceLoadedProject({ sequencer, patternClips: [] } as unknown as LoadedProject))
    expect(state.patterns).toHaveLength(2)
    expect(state.rows).toHaveLength(1)
    expect(state.selectedPatternId).toBe(0)
  })
})

describe('changeTempo', () => {
  const mockInvoke = invoke as jest.Mock
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
    const dispatch = jest.fn()
    expect(await changeTempo(NaN)(dispatch)).toBe(false)
    expect(mockInvoke).not.toHaveBeenCalled()
  })
})

describe('sequencer edits and undo', () => {
  const mockInvoke = invoke as jest.Mock

  beforeEach(() => {
    mockInvoke.mockReset()
    mockInvoke.mockResolvedValue(sequencer)
    resetGestures()
  })

  const sent = () => mockInvoke.mock.calls.map(([command, args]) => [command, args.gesture])

  it('sends every step of one stroke, and every value of one knob drag, as one gesture', async () => {
    const dispatch = jest.fn()

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
    const dispatch = jest.fn()
    await setPatternStep(0, 0, 4, true)(dispatch)
    // a click is a press too, but a mute is always its own step
    beginGesture()
    await setSequencerRowMuted(0, true)(dispatch)

    expect(sent()).toEqual([['set_pattern_step', null], ['set_sequencer_row_muted', undefined]])
  })
})
