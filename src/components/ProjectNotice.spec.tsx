import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { fireEvent, render, screen } from '@testing-library/react'
import projectReducer, { ProjectNotice as Notice, setProjectNotice } from '../state/slices/projectSlice'
import { MissingEffect } from '../types/project'
import ProjectNotice from './ProjectNotice'
import { synthRow } from '../types/sequencer.fixture'

const effect = (track: number | null, name: string): MissingEffect => ({
  track, name, vendor: 'Acme', path: `/x/${name}.vst3`, reason: 'not installed',
})

const setup = (notice: Notice | null) => {
  const store = configureStore({ reducer: { project: projectReducer } })
  store.dispatch(setProjectNotice(notice))
  render(<Provider store={store}><ProjectNotice /></Provider>)
  return store
}

const PLUGINS = /These plugins aren’t installed, so their effects are passing sound straight through/

describe('the project notice', () => {
  it('lists missing samples', () => {
    setup({ kind: 'missing', samples: [{ path: '/gone.wav', reason: 'file not found' }], effects: [] })
    expect(screen.getByText(/1 clip was left out/)).toBeTruthy()
    expect(screen.getByText('/gone.wav')).toBeTruthy()
    expect(screen.queryByText(PLUGINS)).toBeNull()
  })

  it('lists missing effects by track, and the master', () => {
    setup({ kind: 'missing', samples: [], effects: [effect(2, 'Verb'), effect(null, 'Limit')] })
    expect(screen.getByText(PLUGINS)).toBeTruthy()
    expect(screen.getByText(/Verb/).textContent).toBe('Verb (Acme) on Track 3')
    expect(screen.getByText(/Limit/).textContent).toBe('Limit (Acme) on the master')
    expect(screen.queryByText(/clip was left out|clips were left out/)).toBeNull()
  })

  it('lists both when both are missing', () => {
    setup({ kind: 'missing', samples: [{ path: '/gone.wav', reason: 'file not found' }], effects: [effect(0, 'Verb')] })
    expect(screen.getByText(/1 clip was left out/)).toBeTruthy()
    expect(screen.getByText(PLUGINS)).toBeTruthy()
  })

  it('treats missing effects as none', () => {
    // a notice from before effects were kept
    setup({ kind: 'missing', samples: [{ path: '/gone.wav', reason: 'file not found' }] } as unknown as Notice)
    expect(screen.getByText(/1 clip was left out/)).toBeTruthy()
    expect(screen.queryByText(PLUGINS)).toBeNull()
  })

  it('lists missing synths alongside missing effects', () => {
    const synth = synthRow(4, { pluginId: 'vst3:pad', name: 'Pad 2', missing: true }).instrument
    setup({ kind: 'missing', samples: [], effects: [effect(0, 'Verb')], synths: [synth] })
    expect(screen.getByText(PLUGINS)).toBeTruthy()
    expect(screen.getByText(/This instrument isn’t installed, so its notes are silent/)).toBeTruthy()
    expect(screen.getByText('Pad 2 (Acme) in the Note Sequencer')).toBeTruthy()
  })

  it('says what the project left out', () => {
    const dropped = ['Track 2’s instrument was saved the old way, and was left out.', 'Notes in 3 drum patterns were left out.']
    setup({ kind: 'missing', samples: [], effects: [], synths: [], dropped })
    expect(screen.getByText(dropped[0])).toBeTruthy()
    expect(screen.getByText(dropped[1])).toBeTruthy()
    expect(screen.queryByText(PLUGINS)).toBeNull()
  })

  it('is dismissed', () => {
    const store = setup({ kind: 'missing', samples: [], effects: [effect(0, 'Verb')] })
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss' }))
    expect(store.getState().project.notice).toBeNull()
  })
})
