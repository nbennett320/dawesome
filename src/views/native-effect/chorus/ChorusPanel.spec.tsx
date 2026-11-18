import type { Mock } from 'vitest'
import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { normalizedOf, ParamWire } from '../../../types/nativeEffect'
import { param } from '../../../types/nativeEffect.fixture'
import type { EffectParamsApi } from '../useEffectParams'
import ChorusPanel from './ChorusPanel'

vi.mock('../../../state/gesture', () => ({ currentGesture: () => null, newGesture: () => 1 }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => ({ written: 0, rate: 48000, slots: Array.from({ length: 32 }, () => 0) })) }))

const area = { width: 800, height: 400 }
const NOTES = ['1/16 T', '1/16', '1/8 T', '1/16 D', '1/8', '1/4 T', '1/8 D', '1/4', '1/2 T', '1/4 D', '1/2', '1/1 T', '1/2 D', '1/1', '1/1 D', '2/1', '4/1', '8/1']

// the Chorus's parameters, as the engine describes them
const chorusParams = (v: { sync?: boolean, bloom?: number } = {}): ParamWire[] => {
  const params: ParamWire[] = []
  const add = (key: string, change: Partial<ParamWire>, plain: number, fallback = plain) => {
    const id = params.length
    const p = param(id, { key, ...change })
    params.push({ ...p, defaultNormalized: normalizedOf(p, fallback), value: { id, value: plain, normalized: normalizedOf(p, plain), text: `${plain}` } })
  }
  add('sync', { name: 'Sync', group: 'Motion', stepCount: 1 }, v.sync ? 1 : 0, 0)
  add('note', { name: 'Note', group: 'Motion', max: 17, stepCount: 17, labels: NOTES }, 13)
  add('rate', { name: 'Rate', group: 'Motion', unit: 'Hz', min: 0.03, max: 10, taper: 'log' }, 0.8)
  add('depth', { name: 'Depth', group: 'Motion', unit: '%', min: 0, max: 100 }, 40)
  add('shape', { name: 'Shape', group: 'Motion', max: 1, stepCount: 1, labels: ['Sine', 'Triangle'] }, 1)
  add('delay', { name: 'Delay', group: 'Voices', unit: 'ms', min: 3, max: 40, taper: 'log' }, 12)
  add('voices', { name: 'Voices', group: 'Voices', max: 3, stepCount: 3, labels: ['1', '2', '3', '4'] }, 1)
  add('spread', { name: 'Spread', group: 'Voices', unit: '%', min: 0, max: 100 }, 70)
  add('lowcut', { name: 'Low cut', group: 'Tone', unit: 'Hz', min: 20, max: 1000, taper: 'log' }, 120)
  add('tone', { name: 'Tone', group: 'Tone', unit: 'Hz', min: 1000, max: 20000, taper: 'log' }, 9000)
  add('mix', { name: 'Mix', group: 'Mix', unit: '%', min: 0, max: 100 }, 50)
  add('bloom', { name: 'Bloom', group: 'Experimental', unit: '%', min: 0, max: 100 }, v.bloom ?? 0, 0)
  add('drift', { name: 'Drift', group: 'Experimental', unit: '%', min: 0, max: 100 }, 0)
  add('prism', { name: 'Prism', group: 'Experimental', unit: '%', min: 0, max: 100 }, 0)
  return params
}

let set: Mock<EffectParamsApi['set']>

const setup = (params: ParamWire[]) => {
  set = vi.fn<EffectParamsApi['set']>()
  const api: EffectParamsApi = { status: { kind: 'ready', effect: 'chorus' }, params, set, setText: vi.fn() }
  render(<ChorusPanel api={api} slotId={3} active={false} />)
  return screen.getByLabelText(/^Chorus voices/)
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

describe('the Chorus’s panel', () => {
  it('shows its groups, with Rate while free and Note while synced', () => {
    setup(chorusParams())
    ;['Motion', 'Voices', 'Tone', 'Mix', 'Experimental'].forEach(name => expect(screen.getByRole('region', { name })).toBeTruthy())
    expect(screen.queryAllByLabelText(/Motion Rate/).length).toBeGreaterThan(0)
    expect(screen.queryAllByLabelText(/Motion Note/)).toEqual([])
  })

  it('shows Note while synced', () => {
    setup(chorusParams({ sync: true }))
    expect(screen.queryAllByLabelText(/Motion Note/).length).toBeGreaterThan(0)
    expect(screen.queryAllByLabelText(/Motion Rate/)).toEqual([])
  })

  it('sets the voices with a key', () => {
    setup(chorusParams())
    fireEvent.click(screen.getByRole('button', { name: 'Voices 4' }))
    expect(set).toHaveBeenCalledWith(6, 1)
  })

  it('turns every experimental parameter off with one key, which is only live while one is on', () => {
    setup(chorusParams())
    const off = screen.getByRole('button', { name: /Turn every experimental parameter off/ }) as HTMLButtonElement
    expect(off.disabled).toBe(true)
  })

  it('turns them off', () => {
    setup(chorusParams({ bloom: 60 }))
    fireEvent.click(screen.getByRole('button', { name: /Turn every experimental parameter off/ }))
    expect(set.mock.calls.map(([id, n]) => [id, n])).toEqual([[11, 0], [12, 0], [13, 0]])
  })

  it('sets the rate dragging across and the depth dragging up', () => {
    const view = setup(chorusParams())
    view.getBoundingClientRect = () => ({ left: 0, top: 0, width: area.width, height: area.height }) as DOMRect
    fireEvent.pointerDown(view, { button: 0, clientX: 100, clientY: 200, pointerId: 1 })
    fireEvent.pointerMove(view, { clientX: 180, clientY: 160, pointerId: 1 })
    const rate = normalizedOf({ min: 0.03, max: 10, taper: 'log' }, 0.8)
    expect(set).toHaveBeenCalledWith(2, expect.closeTo(rate + 80 / area.width, 6))
    expect(set).toHaveBeenCalledWith(3, expect.closeTo(0.4 + 40 / area.height, 6))
  })
})
