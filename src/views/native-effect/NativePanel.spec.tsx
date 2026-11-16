import type { Mock } from 'vitest'
import React from 'react'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { EffectParams, ParamValue } from '../../types/nativeEffect'
import { param } from '../../types/nativeEffect.spec'
import NativePanel from './NativePanel'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))

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

const setup = async () => {
  render(<NativePanel slotId={7} />)
  await act(async () => {})
}

beforeEach(() => {
  vi.resetAllMocks()
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
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 0, normalized: 1 })
  })

  it('picks from a list', async () => {
    await setup()
    fireEvent.change(screen.getByRole('combobox', { name: 'Band 1 Shape' }), { target: { value: '2' } })
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 1, normalized: 1 })
  })

  it('turns a knob, showing where it went and then what the engine says', async () => {
    await setup()
    const knob = screen.getByRole('slider', { name: 'Output' })
    act(() => { fireEvent.keyDown(knob, { key: 'End' }) })
    expect(knob.getAttribute('aria-valuenow')).toBe('1')
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 3, normalized: 1 })
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
    expect(mockInvoke).toHaveBeenCalledWith('set_effect_param', { slotId: 7, id: 2, normalized: 0.13265 })
    expect(mockInvoke).toHaveBeenCalledTimes(2)
    fireEvent.doubleClick(screen.getByText('Freq'))
    expect(mockInvoke).toHaveBeenLastCalledWith('set_effect_param', { slotId: 7, id: 2, normalized: 0.13265 })
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

  it('says so when its parameters can’t be had', async () => {
    answers.get_effect_params = () => { throw new Error('That effect isn’t one of Dawesome’s own') }
    await setup()
    expect(screen.getByRole('alert').textContent).toContain('isn’t one of Dawesome’s own')
  })
})
