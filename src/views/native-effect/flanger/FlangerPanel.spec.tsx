import type { Mock } from 'vitest'
import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { normalizedOf, ParamWire } from '../../../types/nativeEffect'
import { param } from '../../../types/nativeEffect.fixture'
import type { EffectParamsApi } from '../useEffectParams'
import FlangerPanel from './FlangerPanel'

vi.mock('../../../state/gesture', () => ({ currentGesture: () => null, newGesture: () => 1 }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => ({ written: 0, rate: 48000, slots: Array.from({ length: 32 }, () => 0) })) }))

const area = { width: 800, height: 400 }
const NOTES = ['1/16 T', '1/16', '1/8 T', '1/16 D', '1/8', '1/4 T', '1/8 D', '1/4', '1/2 T', '1/4 D', '1/2', '1/1 T', '1/2 D', '1/1', '1/1 D', '2/1', '4/1', '8/1']
const TUNINGS = ['Off', 'C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

// the Flanger's parameters, as the engine describes them
const flangerParams = (v: { tune?: number } = {}): ParamWire[] => {
  const params: ParamWire[] = []
  const add = (key: string, change: Partial<ParamWire>, plain: number, fallback = plain) => {
    const id = params.length
    const p = param(id, { key, ...change })
    params.push({ ...p, defaultNormalized: normalizedOf(p, fallback), value: { id, value: plain, normalized: normalizedOf(p, plain), text: `${plain}` } })
  }
  add('sync', { name: 'Sync', group: 'Sweep', stepCount: 1 }, 0)
  add('note', { name: 'Note', group: 'Sweep', max: 17, stepCount: 17, labels: NOTES }, 15)
  add('rate', { name: 'Rate', group: 'Sweep', unit: 'Hz', min: 0.01, max: 10, taper: 'log' }, 0.2)
  add('depth', { name: 'Depth', group: 'Sweep', unit: '%', min: 0, max: 100 }, 70)
  add('shape', { name: 'Shape', group: 'Sweep', max: 1, stepCount: 1, labels: ['Sine', 'Triangle'] }, 0)
  add('manual', { name: 'Manual', group: 'Comb', unit: 'ms', min: 0.2, max: 10, taper: 'log' }, 2)
  add('feedback', { name: 'Feedback', group: 'Comb', unit: '%', min: -95, max: 95 }, 60)
  add('tone', { name: 'Tone', group: 'Comb', unit: 'Hz', min: 1000, max: 20000, taper: 'log' }, 12000)
  add('zero', { name: 'Through-zero', group: 'Comb', stepCount: 1 }, 0)
  add('spread', { name: 'Spread', group: 'Stereo', unit: '%', min: 0, max: 100 }, 25)
  add('mix', { name: 'Mix', group: 'Mix', unit: '%', min: 0, max: 100 }, 50)
  add('barberpole', { name: 'Barberpole', group: 'Experimental', unit: '%', min: -100, max: 100 }, 0)
  add('tune', { name: 'Tune', group: 'Experimental', max: 12, stepCount: 12, labels: TUNINGS }, v.tune ?? 0, 0)
  add('envelope', { name: 'Envelope', group: 'Experimental', unit: '%', min: -100, max: 100 }, 0)
  return params
}

let set: Mock<EffectParamsApi['set']>

const setup = (params: ParamWire[]) => {
  set = vi.fn<EffectParamsApi['set']>()
  const api: EffectParamsApi = { status: { kind: 'ready', effect: 'flanger' }, params, set, setText: vi.fn() }
  render(<FlangerPanel api={api} slotId={3} active={false} />)
  return screen.getByLabelText(/^Flanger comb/)
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

describe('the Flanger’s panel', () => {
  it('shows its groups', () => {
    setup(flangerParams())
    ;['Sweep', 'Comb', 'Stereo', 'Mix', 'Experimental'].forEach(name => expect(screen.getByRole('region', { name })).toBeTruthy())
    expect(screen.queryAllByLabelText(/Comb Through-zero/).length).toBeGreaterThan(0)
  })

  it('turns Tune (and the rest) off with the experimental key', () => {
    setup(flangerParams({ tune: 10 }))
    fireEvent.click(screen.getByRole('button', { name: /Turn every experimental parameter off/ }))
    expect(set.mock.calls.map(([id, n]) => [id, n])).toEqual([[11, 0.5], [12, 0], [13, 0.5]])
  })

  it('sets Manual dragging across and the feedback dragging up', () => {
    const view = setup(flangerParams())
    view.getBoundingClientRect = () => ({ left: 0, top: 0, width: area.width, height: area.height }) as DOMRect
    fireEvent.pointerDown(view, { button: 0, clientX: 100, clientY: 200, pointerId: 1 })
    fireEvent.pointerMove(view, { clientX: 140, clientY: 180, pointerId: 1 })
    const manual = normalizedOf({ min: 0.2, max: 10, taper: 'log' }, 2)
    expect(set).toHaveBeenCalledWith(5, expect.closeTo(manual + 40 / area.width, 6))
    expect(set).toHaveBeenCalledWith(6, expect.closeTo((60 + 95) / 190 + 20 / area.height, 6))
  })
})
