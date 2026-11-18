import type { Mock } from 'vitest'
import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { normalizedOf, ParamWire } from '../../../types/nativeEffect'
import { param } from '../../../types/nativeEffect.fixture'
import type { EffectParamsApi } from '../useEffectParams'
import PhaserPanel from './PhaserPanel'

vi.mock('../../../state/gesture', () => ({ currentGesture: () => null, newGesture: () => 1 }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(async () => ({ written: 0, rate: 48000, slots: Array.from({ length: 32 }, () => 0) })) }))

const area = { width: 800, height: 400 }
const NOTES = ['1/16 T', '1/16', '1/8 T', '1/16 D', '1/8', '1/4 T', '1/8 D', '1/4', '1/2 T', '1/4 D', '1/2', '1/1 T', '1/2 D', '1/1', '1/1 D', '2/1', '4/1', '8/1']

// the Phaser's parameters, as the engine describes them
const phaserParams = (v: { vibe?: boolean } = {}): ParamWire[] => {
  const params: ParamWire[] = []
  const add = (key: string, change: Partial<ParamWire>, plain: number, fallback = plain) => {
    const id = params.length
    const p = param(id, { key, ...change })
    params.push({ ...p, defaultNormalized: normalizedOf(p, fallback), value: { id, value: plain, normalized: normalizedOf(p, plain), text: `${plain}` } })
  }
  add('sync', { name: 'Sync', group: 'Sweep', stepCount: 1 }, 0)
  add('note', { name: 'Note', group: 'Sweep', max: 17, stepCount: 17, labels: NOTES }, 13)
  add('rate', { name: 'Rate', group: 'Sweep', unit: 'Hz', min: 0.01, max: 10, taper: 'log' }, 0.5)
  add('depth', { name: 'Depth', group: 'Sweep', unit: '%', min: 0, max: 100 }, 80)
  add('shape', { name: 'Shape', group: 'Sweep', max: 1, stepCount: 1, labels: ['Sine', 'Triangle'] }, 0)
  add('center', { name: 'Center', group: 'Notches', unit: 'Hz', min: 100, max: 4000, taper: 'log' }, 700)
  add('stages', { name: 'Stages', group: 'Notches', max: 5, stepCount: 5, labels: ['2', '4', '6', '8', '10', '12'] }, 2)
  add('feedback', { name: 'Feedback', group: 'Notches', unit: '%', min: -95, max: 95 }, 50)
  add('mode', { name: 'Mode', group: 'Notches', max: 1, stepCount: 1, labels: ['Classic', 'Vibe'] }, v.vibe ? 1 : 0, 0)
  add('spread', { name: 'Spread', group: 'Stereo', unit: '%', min: 0, max: 100 }, 50)
  add('mix', { name: 'Mix', group: 'Mix', unit: '%', min: 0, max: 100 }, 50)
  add('melt', { name: 'Melt', group: 'Experimental', unit: '%', min: 0, max: 100 }, 0)
  add('barberpole', { name: 'Barberpole', group: 'Experimental', unit: '%', min: -100, max: 100 }, 0)
  add('envelope', { name: 'Envelope', group: 'Experimental', unit: '%', min: -100, max: 100 }, 0)
  return params
}

let set: Mock<EffectParamsApi['set']>

const setup = (params: ParamWire[]) => {
  set = vi.fn<EffectParamsApi['set']>()
  const api: EffectParamsApi = { status: { kind: 'ready', effect: 'phaser' }, params, set, setText: vi.fn() }
  render(<PhaserPanel api={api} slotId={3} active={false} />)
  return screen.getByLabelText(/^Phaser notches/)
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

describe('the Phaser’s panel', () => {
  it('shows its groups, and sets the stages and mode with keys', () => {
    setup(phaserParams())
    ;['Sweep', 'Notches', 'Stereo', 'Mix', 'Experimental'].forEach(name => expect(screen.getByRole('region', { name })).toBeTruthy())
    fireEvent.click(screen.getByRole('button', { name: 'Stages 8' }))
    expect(set).toHaveBeenCalledWith(6, 0.6)
    fireEvent.click(screen.getByRole('button', { name: 'Mode Vibe' }))
    expect(set).toHaveBeenCalledWith(8, 1)
  })

  it('dims the stages and shape in Vibe mode, which sets its own', () => {
    setup(phaserParams({ vibe: true }))
    expect(screen.getByRole('group', { name: 'Stages' }).parentElement?.className).toMatch(/opacity-40/)
    expect(screen.getByRole('group', { name: 'Mode' }).parentElement?.className).not.toMatch(/opacity-40/)
  })

  it('sets the centre dragging across and the feedback dragging up', () => {
    const view = setup(phaserParams())
    view.getBoundingClientRect = () => ({ left: 0, top: 0, width: area.width, height: area.height }) as DOMRect
    fireEvent.pointerDown(view, { button: 0, clientX: 100, clientY: 200, pointerId: 1 })
    fireEvent.pointerMove(view, { clientX: 160, clientY: 200, pointerId: 1 })
    const center = normalizedOf({ min: 100, max: 4000, taper: 'log' }, 700)
    expect(set).toHaveBeenCalledWith(5, expect.closeTo(center + 60 / area.width, 6))
  })
})
