import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, render, screen } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import playlistReducer from '../../state/slices/playlistSlice'
import projectReducer from '../../state/slices/projectSlice'
import mixerReducer from '../../state/slices/mixerSlice'
import windowReducer, { selectFocusedView } from '../../state/slices/windowSlice'
import effectsReducer from '../../state/slices/effectsSlice'
import sequencerReducer from '../../state/slices/sequencerSlice'
import { DragItem, beginSampleDrag } from '../../state/sampleDrag'
import { ProjectView } from '../../types/project'
import { View } from '../../types/ui'
import { DROP_CHANNEL, useChannelDrop } from './useChannelDrop'
import { pattern, sequencerOf, synthRow } from '../../types/sequencer.fixture'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
const mockInvoke = invoke as Mock

// two strips, track 2's and the master's, as the mixer marks them
const Strips = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const target = useChannelDrop(ref)
  return (
    <div ref={ref}>
      <div {...{ [DROP_CHANNEL]: '2' }}>Track 3</div>
      <div {...{ [DROP_CHANNEL]: 'master' }}>Master</div>
      <span data-testid='target'>{String(target)}</span>
    </div>
  )
}

const project: ProjectView = {
  tempo: 120,
  clips: [],
  patternClips: [{ id: 40, patternId: 5, trackNumber: 2, startBeat: 6, window: null }],
  mixer: { tracks: [], master: { volumeDb: 0, pan: 0, mute: false, solo: false } },
  // the new synth row (9, in slot 109) and its pattern
  sequencer: sequencerOf({ rows: [synthRow(9, { pluginId: 'vst3:pad', name: 'Pad' })], patterns: [pattern(5)] }),
}

const setup = () => {
  const store = configureStore({
    reducer: {
      playlist: playlistReducer,
      project: projectReducer,
      mixer: mixerReducer,
      window: windowReducer,
      effects: effectsReducer,
      sequencer: sequencerReducer,
    },
  })
  render(<Provider store={store}><Strips /></Provider>)
  return store
}

// dragged from the sidebar, let go over a strip
const dropOn = async (item: DragItem, strip: string) => {
  document.elementFromPoint = () => screen.getByText(strip)
  beginSampleDrag(item, { button: 0, clientX: 0, clientY: 0 })
  window.dispatchEvent(new MouseEvent('mousemove', { clientX: 40, clientY: 40 }))
  await act(async () => {})
  const shown = screen.getByTestId('target').textContent
  window.dispatchEvent(new MouseEvent('mouseup', { clientX: 40, clientY: 40 }))
  await act(async () => {})
  return shown
}

const instrument: DragItem = { kind: 'instrument', pluginId: 'vst3:pad', label: 'Pad' }

beforeEach(() => {
  mockInvoke.mockReset()
  mockInvoke.mockImplementation(async (command: string) => {
    if(command === 'get_playlist_transport') return { positionMs: 3000, beatIntervalMs: 500 }
    if(command === 'add_synth_on_track') return { rowId: 9, patternId: 5, project }
    return undefined
  })
})

describe('an instrument dropped on a mixer strip', () => {
  it('starts a synth row’s pattern on the track at the playhead, and opens the Piano Roll on it', async () => {
    const store = setup()
    expect(await dropOn(instrument, 'Track 3')).toBe('2')

    expect(mockInvoke).toHaveBeenCalledWith('add_synth_on_track', { pluginId: 'vst3:pad', trackNumber: 2, dropBeat: 6 })
    const state = store.getState()
    expect(selectFocusedView(state as never)).toBe(View.PianoRoll)
    expect(state.sequencer.selected).toBe(5)
    expect(state.sequencer.pianoRollRowId).toBe(9)
    expect(state.playlist.playlistItems.map(item => item.id)).toEqual([40])
  })

  it('is refused by the master', async () => {
    setup()
    expect(await dropOn(instrument, 'Master')).toBe('null')
    expect(mockInvoke).not.toHaveBeenCalledWith('add_synth_on_track', expect.anything())
  })

  it('leaves effects to go on the channel as before', async () => {
    setup()
    await dropOn({ kind: 'effect', pluginId: 'vst3:verb', label: 'Verb' }, 'Master')
    expect(mockInvoke).toHaveBeenCalledWith('add_effect', { track: null, pluginId: 'vst3:verb', index: null })
    expect(mockInvoke).not.toHaveBeenCalledWith('add_synth_on_track', expect.anything())
  })
})
