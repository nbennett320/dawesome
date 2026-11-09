import reducer, { sequencerSlice, selectPattern, setSequencer } from './sequencerSlice'
import { reduceLoadedProject, setPlaying } from './playlistSlice'
import { LoadedProject } from '../../types/project'
import { SequencerState } from '../../types/sequencer'

jest.mock('@tauri-apps/api', () => ({ invoke: jest.fn() }))

const sequencer: SequencerState = {
  rows: [{ id: 0, path: '/drums/kick.wav', volume: 1, muted: false }],
  patterns: [
    { id: 0, name: null, length: 16, steps: { 0: [0, 8] } },
    { id: 4, name: 'fill', length: 32, steps: {} },
  ],
}

const init = () => reducer(undefined, { type: 'init' })

describe('sequencerSlice', () => {
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
    state = reducer(state, setSequencer({ rows: [], patterns: [] }))
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
    let state = reducer(init(), setSequencer({ rows: [], patterns: [{ id: 9, name: null, length: 16, steps: {} }] }))
    state = reducer(state, selectPattern(9))

    state = reducer(state, reduceLoadedProject({ sequencer, patternClips: [] } as unknown as LoadedProject))
    expect(state.patterns).toHaveLength(2)
    expect(state.rows).toHaveLength(1)
    expect(state.selectedPatternId).toBe(0)
  })
})
