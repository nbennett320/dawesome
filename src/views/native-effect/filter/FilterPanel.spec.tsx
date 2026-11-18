import type { Mock } from 'vitest'
import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { normalizedOf, ParamWire } from '../../../types/nativeEffect'
import { param } from '../../../types/nativeEffect.fixture'
import type { EffectParamsApi } from '../useEffectParams'
import { freqToX } from './draw'
import FilterPanel from './FilterPanel'

vi.mock('../../../state/gesture', () => ({ currentGesture: () => null, newGesture: () => 1 }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => ({ written: 0, rate: 48000, slots: Array.from({ length: 32 }, () => 0) })) }))

const area = { width: 800, height: 400 }
const NOTES = ['1/16 T', '1/16', '1/8 T', '1/16 D', '1/8', '1/4 T', '1/8 D', '1/4', '1/2 T', '1/4 D', '1/2', '1/1 T', '1/2 D', '1/1', '1/1 D', '2/1', '4/1', '8/1']
const TUNINGS = ['Off', 'Chromatic', 'C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

// the Filter's parameters, as the engine describes them
const filterParams = (v: { mode?: number, vowel?: number } = {}): ParamWire[] => {
  const params: ParamWire[] = []
  const add = (key: string, change: Partial<ParamWire>, plain: number, fallback = plain) => {
    const id = params.length
    const p = param(id, { key, ...change })
    params.push({ ...p, defaultNormalized: normalizedOf(p, fallback), value: { id, value: plain, normalized: normalizedOf(p, plain), text: `${plain}` } })
  }
  add('type', { name: 'Type', group: 'Filter', max: 2, stepCount: 2, labels: ['Low pass', 'High pass', 'Band pass'] }, 0)
  add('slope', { name: 'Slope', group: 'Filter', max: 1, stepCount: 1, labels: ['12 dB', '24 dB'] }, 1)
  add('cutoff', { name: 'Cutoff', group: 'Filter', unit: 'Hz', min: 20, max: 20000, taper: 'log' }, 2000)
  add('resonance', { name: 'Resonance', group: 'Filter', unit: '%', min: 0, max: 100 }, 20)
  add('drive', { name: 'Drive', group: 'Filter', unit: 'dB', min: 0, max: 24 }, 0)
  add('mode', { name: 'Mode', group: 'Motion', max: 3, stepCount: 3, labels: ['Off', 'LFO', 'Envelope', 'Dynamic'] }, v.mode ?? 0, 0)
  add('floor', { name: 'Floor', group: 'Motion', unit: 'Hz', min: 20, max: 20000, taper: 'log' }, 200)
  add('sync', { name: 'Sync', group: 'LFO', stepCount: 1 }, 1)
  add('note', { name: 'Note', group: 'LFO', max: 17, stepCount: 17, labels: NOTES }, 7)
  add('rate', { name: 'Rate', group: 'LFO', unit: 'Hz', min: 0.03, max: 20, taper: 'log' }, 1)
  add('shape', { name: 'Shape', group: 'LFO', max: 5, stepCount: 5, labels: ['Sine', 'Triangle', 'Saw up', 'Saw down', 'Square', 'S&H'] }, 0)
  add('attack', { name: 'Attack', group: 'Envelope', unit: 'ms', min: 0.5, max: 500, taper: 'log' }, 5)
  add('decay', { name: 'Decay', group: 'Envelope', unit: 'ms', min: 10, max: 4000, taper: 'log' }, 300)
  add('sensitivity', { name: 'Sensitivity', group: 'Envelope', unit: '%', min: 0, max: 100 }, 50)
  add('mix', { name: 'Mix', group: 'Output', unit: '%', min: 0, max: 100 }, 100)
  add('output', { name: 'Output', group: 'Output', unit: 'dB', min: -24, max: 12 }, 0)
  add('vowel', { name: 'Vowel', group: 'Experimental', unit: '%', min: 0, max: 100 }, v.vowel ?? 0, 0)
  add('spin', { name: 'Spin', group: 'Experimental', unit: '%', min: 0, max: 100 }, 0)
  add('tune', { name: 'Tune', group: 'Experimental', max: 13, stepCount: 13, labels: TUNINGS }, 0)
  return params
}

let set: Mock<EffectParamsApi['set']>

const setup = (params: ParamWire[]) => {
  set = vi.fn<EffectParamsApi['set']>()
  const api: EffectParamsApi = { status: { kind: 'ready', effect: 'filter' }, params, set, setText: vi.fn() }
  render(<FilterPanel api={api} slotId={3} active={false} />)
  const view = screen.getByLabelText(/^Filter curve/)
  view.getBoundingClientRect = () => ({ left: 0, top: 0, width: area.width, height: area.height }) as DOMRect
  return view
}

beforeAll(() => {
  applyTheme(slateLightTheme)
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => area.width })
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => area.height })
  HTMLCanvasElement.prototype.getContext = (() => null) as never
})

afterAll(() => {
  delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientWidth
  delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientHeight
})

const shown = (label: RegExp) => screen.queryAllByLabelText(label).length > 0

describe('the Filter’s panel', () => {
  it('shows only Cutoff while Off, and the mode’s own controls once one is on', () => {
    setup(filterParams())
    ;['Filter', 'Motion', 'Output', 'Experimental'].forEach(name => expect(screen.getByRole('region', { name })).toBeTruthy())
    expect(shown(/Motion Floor/)).toBe(false)
    expect(shown(/LFO Note/)).toBe(false)
    fireEvent.click(screen.getByRole('button', { name: 'Mode LFO' }))
    expect(set).toHaveBeenCalledWith(5, 1 / 3)
  })

  it('shows the LFO’s controls in LFO mode', () => {
    setup(filterParams({ mode: 1 }))
    expect(shown(/Motion Floor/)).toBe(true)
    expect(shown(/LFO Note/)).toBe(true)
    expect(shown(/LFO Shape/)).toBe(true)
    expect(shown(/Envelope Attack/)).toBe(false)
  })

  it('calls Decay Release in Dynamic mode', () => {
    setup(filterParams({ mode: 3 }))
    expect(shown(/Envelope Release/)).toBe(true)
    expect(shown(/Envelope Decay/)).toBe(false)
  })

  it('drags the cutoff across and the resonance up, anywhere off the floor’s handle', () => {
    const view = setup(filterParams({ mode: 1 }))
    fireEvent.pointerDown(view, { button: 0, clientX: 400, clientY: 200, pointerId: 1 })
    fireEvent.pointerMove(view, { clientX: 448, clientY: 160, pointerId: 1 })
    const cutoff = normalizedOf({ min: 20, max: 20000, taper: 'log' }, 2000)
    expect(set).toHaveBeenCalledWith(2, expect.closeTo(cutoff + 48 / (area.width - 32), 6))
    expect(set).toHaveBeenCalledWith(3, expect.closeTo(0.2 + 40 / area.height, 6))
  })

  it('drags the floor by its handle', () => {
    const view = setup(filterParams({ mode: 1 }))
    const x = freqToX(200, area)
    fireEvent.pointerDown(view, { button: 0, clientX: x, clientY: 50, pointerId: 1 })
    fireEvent.pointerMove(view, { clientX: x + 30, clientY: 50, pointerId: 1 })
    const floor = normalizedOf({ min: 20, max: 20000, taper: 'log' }, 200)
    expect(set).toHaveBeenCalledWith(6, expect.closeTo(floor + 30 / (area.width - 32), 6))
    expect(set.mock.calls.some(([id]) => id === 3)).toBe(false)
  })

  it('turns the experimental ones off', () => {
    setup(filterParams({ vowel: 50 }))
    fireEvent.click(screen.getByRole('button', { name: /Turn every experimental parameter off/ }))
    expect(set.mock.calls.map(([id, n]) => [id, n])).toEqual([[16, 0], [17, 0], [18, 0]])
  })
})
