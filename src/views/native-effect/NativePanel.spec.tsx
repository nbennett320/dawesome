import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import effectsReducer from '../../state/slices/effectsSlice'
import playlistReducer, { reduceProjectView } from '../../state/slices/playlistSlice'
import { ProjectView } from '../../types/project'
import { EffectParams, ParamValue } from '../../types/nativeEffect'
import { param } from '../../types/nativeEffect.fixture'
import NativePanel from './NativePanel'
import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
// the mouse held down (a drag) or not
let gesture: number | null = null
vi.mock('../../state/gesture', () => ({ currentGesture: () => gesture }))

const mockInvoke = invoke as Mock

const value = (id: number, v: number, normalized: number, text: string): ParamValue => ({ id, value: v, normalized, text })

const effect = (): EffectParams => ({
  effect: 'test',
  params: [
    param(0, { name: 'On', group: 'Band 1', stepCount: 1, value: value(0, 0, 0, 'Off') }),
    param(1, {
      name: 'Shape',
      group: 'Band 1',
      min: 0,
      max: 2,
      stepCount: 2,
      labels: ['Bell', 'Notch', 'Low cut'],
      value: value(1, 0, 0, 'Bell'),
    }),
    param(2, {
      name: 'Freq',
      group: 'Band 1',
      unit: 'Hz',
      min: 20,
      max: 20000,
      taper: 'log',
      default: 632.46,
      defaultNormalized: 0.5,
      value: value(2, 1000, 0.566, '1.00 kHz'),
    }),
    param(3, { name: 'Output', group: 'Output', unit: 'dB', min: -24, max: 24, value: value(3, 0, 0.5, '0.0 dB') }),
  ],
})

// what each command answers (a function of its arguments)
let answers: Record<string, (args: Record<string, unknown>) => unknown>

const makeStore = () => configureStore({ reducer: { effects: effectsReducer, playlist: playlistReducer } })
let store: ReturnType<typeof makeStore>

const setup = async () => {
  store = makeStore()
  render(<Provider store={store}><NativePanel slotId={7} /></Provider>)
  await act(async () => {})
}

beforeAll(() => {
  applyTheme(slateLightTheme)
  // jsdom has no canvas: nothing is drawn
  HTMLCanvasElement.prototype.getContext = (() => null) as never
})

beforeEach(() => {
  vi.resetAllMocks()
  gesture = null
  answers = {
    get_effect_params: () => effect(),
    set_effect_param: args => value(args.id as number, 1, args.normalized as number, 'set'),
    set_effect_param_text: args => value(args.id as number, 2500, 0.6, '2.50 kHz'),
  }
  mockInvoke.mockImplementation(async (command: string, args: Record<string, unknown>) => {
    const answer = answers[command]
    if(!answer) throw new Error(`unexpected ${command}`)
    return answer(args)
  })
})

describe('a built-in effect’s generated panel', () => {
  it('draws each group of parameters as a box, in order', async () => {
    await setup()
    expect(mockInvoke).toHaveBeenCalledWith('get_effect_params', { slotId: 7 })
    const groups = screen.getAllByRole('region')
    expect(groups.map(g => g.getAttribute('aria-label'))).toEqual(['Band 1', 'Output'])
    const band = within(groups[0])
    expect(band.getByRole('button', { name: 'Band 1 On' })).toBeTruthy()
    expect(band.getByRole('combobox', { name: 'Band 1 Shape' })).toBeTruthy()
    expect(band.getByRole('slider', { name: 'Band 1 Freq' }).getAttribute('aria-valuetext')).toBe('1.00 kHz')
    expect((band.getByRole('textbox', { name: 'Band 1 Freq value' }) as HTMLInputElement).value).toBe('1.00 kHz')
  })

  it('switches a switch', async () => {
    await setup()
    fireEvent.click(screen.getByRole('button', { name: 'Band 1 On' }))
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 0, normalized: 1, gesture: null })
  })

  it('picks from a list', async () => {
    await setup()
    fireEvent.change(screen.getByRole('combobox', { name: 'Band 1 Shape' }), { target: { value: '2' } })
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 1, normalized: 1, gesture: null })
  })

  it('turns a knob, showing where it went and then what the engine says', async () => {
    await setup()
    const knob = screen.getByRole('slider', { name: 'Output' })
    act(() => { fireEvent.keyDown(knob, { key: 'End' }) })
    expect(knob.getAttribute('aria-valuenow')).toBe('1')
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 3, normalized: 1, gesture: null })
    await act(async () => {})
    expect((screen.getByRole('textbox', { name: 'Output value' }) as HTMLInputElement).value).toBe('set')
  })

  it('puts a knob back exactly to its default on a double-click', async () => {
    // (50 Hz: between the knob's steps, which it mustn't be rounded to)
    const params = effect()
    params.params[2] = { ...params.params[2], default: 50, defaultNormalized: 0.13265 }
    answers.get_effect_params = () => params
    await setup()
    fireEvent.doubleClick(screen.getByRole('slider', { name: 'Band 1 Freq' }))
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 2, normalized: 0.13265, gesture: null })
    expect(mockInvoke).toHaveBeenCalledTimes(2)
    fireEvent.doubleClick(screen.getByText('Freq'))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_effect_param', { slotId: 7, id: 2, normalized: 0.13265, gesture: null })
  })

  it('takes a typed value', async () => {
    await setup()
    const field = screen.getByRole('textbox', { name: 'Band 1 Freq value' }) as HTMLInputElement
    fireEvent.focus(field)
    fireEvent.change(field, { target: { value: '2.5k' } })
    fireEvent.blur(field)
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param_text', { slotId: 7, id: 2, text: '2.5k' })
    await act(async () => {})
    expect(field.value).toBe('2.50 kHz')
  })

  it('keeps only the answer to the latest set when answers come out of order', async () => {
    const pending: Array<(v: ParamValue) => void> = []
    answers.set_effect_param = () => new Promise(resolve => { pending.push(resolve) })
    await setup()
    const knob = screen.getByRole('slider', { name: 'Output' })
    act(() => { fireEvent.keyDown(knob, { key: 'Home' }) })
    act(() => { fireEvent.keyDown(knob, { key: 'End' }) })
    // the later answer comes first, then the earlier one
    await act(async () => { pending[1](value(3, 24, 1, '+24.0 dB')) })
    await act(async () => { pending[0](value(3, -24, 0, '-24.0 dB')) })
    expect((screen.getByRole('textbox', { name: 'Output value' }) as HTMLInputElement).value).toBe('+24.0 dB')
    expect(knob.getAttribute('aria-valuenow')).toBe('1')
  })

  it('sends a drag’s gesture with each set, so the drag is undone as one', async () => {
    await setup()
    gesture = 12
    act(() => { fireEvent.keyDown(screen.getByRole('slider', { name: 'Output' }), { key: 'End' }) })
    expect(mockInvoke).toHaveBeenLastCalledWith('set_effect_param', { slotId: 7, id: 3, normalized: 1, gesture: 12 })
  })

  it('tells the unsaved watch each time a value is set', async () => {
    await setup()
    const before = store.getState().effects.paramEdits
    act(() => { fireEvent.keyDown(screen.getByRole('slider', { name: 'Output' }), { key: 'End' }) })
    await act(async () => {})
    expect(store.getState().effects.paramEdits).toBe(before + 1)
  })

  it('fetches its values again after an undo, without going blank', async () => {
    await setup()
    const undone = effect()
    undone.params[3] = { ...undone.params[3], value: value(3, -12, 0.25, '-12.0 dB') }
    answers.get_effect_params = () => undone
    const project = { tempo: 120, clips: [], patternClips: [], effects: [] } as unknown as ProjectView
    act(() => { store.dispatch(reduceProjectView({ project, selected: null })) })
    // still showing what it had while it asks
    expect(screen.queryByText('Loading…')).toBeNull()
    await act(async () => {})
    expect((screen.getByRole('textbox', { name: 'Output value' }) as HTMLInputElement).value).toBe('-12.0 dB')
    expect(mockInvoke.mock.calls.filter(([name]) => name === 'get_effect_params')).toHaveLength(2)
  })

  it('gives the EQ its own panel, with its curve', async () => {
    answers.get_effect_params = () => ({ ...effect(), effect: 'eq' })
    await setup()
    expect(screen.getByLabelText(/^EQ curve/)).toBeTruthy()
    expect(screen.queryAllByRole('region').map(r => r.getAttribute('aria-label'))).not.toContain('Band 1')
  })

  it('says so when its parameters can’t be had', async () => {
    answers.get_effect_params = () => { throw new Error('That effect isn’t one of Dawesome’s own') }
    await setup()
    expect(screen.getByRole('alert').textContent).toContain('isn’t one of Dawesome’s own')
  })
})
