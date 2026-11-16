import type { Mock } from 'vitest'
import React from 'react'
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { normalizedOf, ParamWire } from '../../../types/nativeEffect'
import { param } from '../../../types/nativeEffect.fixture'
import type { EffectParamsApi } from '../useEffectParams'
import EqPanel from './EqPanel'
import { dbToY, freqToX, xToFreqNormalized } from './geometry'
import { BANDS, FREQ, GAIN, ON, OUTPUT, paramId, Q, SHAPE, Shape } from './response'

// every new undo step the panel makes
let gestures = 0
vi.mock('../../../state/gesture', () => ({
  currentGesture: () => null,
  newGesture: () => { gestures += 1; return 100 + gestures },
}))

const SHAPES = ['Bell', 'Low shelf', 'High shelf', 'Low cut', 'High cut', 'Notch', 'Band-pass']
const area = { width: 800, height: 400 }

// the EQ's parameters, as the engine describes them, with some bands set
const eqParams = (bands: Record<number, { shape?: Shape, freq?: number, gain?: number, q?: number }>): ParamWire[] => {
  const params: ParamWire[] = []
  const add = (change: Partial<ParamWire>, plain: number) => {
    const id = params.length
    const p = param(id, change)
    params.push({ ...p, value: { id, value: plain, normalized: normalizedOf(p, plain), text: `${plain}` } })
  }
  for(let b = 0; b < BANDS; b += 1) {
    const set = bands[b]
    const group = `Band ${b + 1}`
    add({ name: 'On', group, stepCount: 1 }, set ? 1 : 0)
    add({ name: 'Shape', group, max: 6, stepCount: 6, labels: SHAPES }, set?.shape ?? Shape.Bell)
    add({ name: 'Freq', group, unit: 'Hz', min: 20, max: 20000, taper: 'log' }, set?.freq ?? 1000)
    add({ name: 'Gain', group, unit: 'dB', min: -24, max: 24 }, set?.gain ?? 0)
    add({ name: 'Q', group, min: 0.1, max: 18, taper: 'log', default: 0.707, defaultNormalized: 0.4 }, set?.q ?? 0.707)
  }
  add({ name: 'Output', group: 'Output', unit: 'dB', min: -24, max: 24 }, 0)
  return params
}

let set: Mock<EffectParamsApi['set']>

const setup = (params: ParamWire[]) => {
  set = vi.fn<EffectParamsApi['set']>()
  const api: EffectParamsApi = { status: { kind: 'ready', effect: 'eq' }, params, set, setText: vi.fn() }
  render(<EqPanel api={api} slotId={7} active={false} />)
  return screen.getByLabelText(/^EQ curve/)
}

// the calls to set, as [id, normalized, gesture]
const sets = () => set.mock.calls.map(([id, n, gesture]) => [id, Number(n.toFixed(4)), gesture])

beforeAll(() => {
  applyTheme(slateLightTheme)
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => area.width })
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => area.height })
  // jsdom has no canvas: nothing is drawn
  HTMLCanvasElement.prototype.getContext = (() => null) as never
})

afterAll(() => {
  delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientWidth
  delete (HTMLElement.prototype as unknown as Record<string, unknown>).clientHeight
})

beforeEach(() => {
  gestures = 0
})

// a band at 1 kHz, +6 dB (band 3), and a low cut at 80 Hz (band 1)
const twoBands = () => eqParams({ 2: { freq: 1000, gain: 6 }, 0: { shape: Shape.LowCut, freq: 80 } })
const dot = (freq: number, db: number) => ({ clientX: freqToX(freq, area), clientY: dbToY(db, area) })

describe('the EQ’s panel', () => {
  it('shows its chips, the first band that’s on, and the output', () => {
    setup(twoBands())
    const chips = within(screen.getByRole('group', { name: 'Bands' })).getAllByRole('button')
    expect(chips).toHaveLength(8)
    expect(chips[0].getAttribute('aria-label')).toBe('Band 1')
    expect(chips[1].getAttribute('aria-label')).toBe('Band 2 (off)')
    expect(chips[0].getAttribute('aria-pressed')).toBe('true')
    expect(screen.getByRole('slider', { name: 'Band 1 Freq' })).toBeTruthy()
    expect(screen.getByRole('slider', { name: 'Output' })).toBeTruthy()
  })

  it('selects a band from its chip, an off one too', () => {
    setup(twoBands())
    fireEvent.click(screen.getByRole('button', { name: 'Band 6 (off)' }))
    expect(screen.getByRole('region', { name: 'Band 6' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Band 6 On' })).toBeTruthy()
  })

  it('adds a bell where empty space is double-clicked, as one undo step', () => {
    const curve = setup(twoBands())
    fireEvent.doubleClick(curve, dot(500, 6))
    // band 2 is the first free one
    expect(sets()).toEqual([
      [paramId(1, SHAPE), 0, 101],
      [paramId(1, FREQ), Number(xToFreqNormalized(freqToX(500, area), area).toFixed(4)), 101],
      [paramId(1, GAIN), 0.625, 101],
      [paramId(1, Q), 0.4, 101],
      [paramId(1, ON), 1, 101],
    ])
    expect(screen.getByRole('region', { name: 'Band 2' })).toBeTruthy()
  })

  it('adds a cut at either end, flat', () => {
    const curve = setup(eqParams({}))
    fireEvent.doubleClick(curve, dot(25, 10))
    expect(sets()).toContainEqual([paramId(0, SHAPE), 0.5, 101])
    expect(sets()).toContainEqual([paramId(0, GAIN), 0.5, 101])
    set.mockClear()
    fireEvent.doubleClick(curve, dot(18000, -10))
    // (the panel's values don't change in this test, so band 1 is still free)
    expect(sets()).toContainEqual([paramId(0, SHAPE), Number((4 / 6).toFixed(4)), 102])
  })

  it('adds nothing when all 8 bands are on', () => {
    const all = Object.fromEntries(Array.from({ length: 8 }, (_, b) => [b, { freq: 100 * (b + 1) }]))
    const curve = setup(eqParams(all))
    fireEvent.doubleClick(curve, dot(15000, 12))
    expect(set).not.toHaveBeenCalled()
  })

  it('moves a band’s frequency and gain by dragging its dot', () => {
    const curve = setup(twoBands())
    const from = dot(1000, 6)
    fireEvent.pointerDown(curve, { ...from, button: 0 })
    expect(screen.getByRole('region', { name: 'Band 3' })).toBeTruthy()
    fireEvent.pointerMove(curve, { clientX: from.clientX + 80, clientY: from.clientY - 47 })
    fireEvent.pointerUp(curve)
    const [freq, gain] = [sets()[0], sets()[1]]
    const start = normalizedOf({ min: 20, max: 20000, taper: 'log' }, 1000)
    expect(freq[0]).toBe(paramId(2, FREQ))
    expect(freq[1]).toBeCloseTo(start + 0.1, 3)
    expect(gain[0]).toBe(paramId(2, GAIN))
    expect(gain[1]).toBeCloseTo(0.625 + 47 / (400 - 24), 3)
    // and stops when it's let go
    set.mockClear()
    fireEvent.pointerMove(curve, { clientX: 10, clientY: 10 })
    expect(set).not.toHaveBeenCalled()
  })

  it('moves a cut sideways only', () => {
    const curve = setup(twoBands())
    fireEvent.pointerDown(curve, { ...dot(80, 0), button: 0 })
    fireEvent.pointerMove(curve, { clientX: dot(80, 0).clientX + 40, clientY: 50 })
    expect(sets().map(([id]) => id)).toEqual([paramId(0, FREQ)])
  })

  it('moves a quarter as far with shift held', () => {
    const curve = setup(twoBands())
    const from = dot(1000, 6)
    fireEvent.pointerDown(curve, { ...from, button: 0, shiftKey: true })
    fireEvent.pointerMove(curve, { clientX: from.clientX + 80, clientY: from.clientY, shiftKey: true })
    const start = normalizedOf({ min: 20, max: 20000, taper: 'log' }, 1000)
    expect(sets()[0][1]).toBeCloseTo(start + 0.025, 3)
  })

  it('does nothing when a press misses every dot', () => {
    const curve = setup(twoBands())
    fireEvent.pointerDown(curve, { clientX: 700, clientY: 380, button: 0 })
    fireEvent.pointerMove(curve, { clientX: 600, clientY: 300 })
    expect(set).not.toHaveBeenCalled()
  })

  it('turns Q by scrolling on a dot, a burst of scrolling as one step', () => {
    const curve = setup(twoBands())
    // (the scroll is kept from the pane: false is "prevented")
    expect(fireEvent.wheel(curve, { ...dot(1000, 6), deltaY: -100 })).toBe(false)
    fireEvent.wheel(curve, { ...dot(1000, 6), deltaY: -100 })
    const q = normalizedOf({ min: 0.1, max: 18, taper: 'log' }, 0.707)
    expect(sets()).toEqual([
      [paramId(2, Q), Number((q + 0.15).toFixed(4)), 101],
      [paramId(2, Q), Number((q + 0.15).toFixed(4)), 101],
    ])
    // not on a dot: nothing, and the pane scrolls
    expect(fireEvent.wheel(curve, { clientX: 700, clientY: 380, deltaY: -100 })).toBe(true)
    expect(set).toHaveBeenCalledTimes(2)
  })

  it('changes a band’s shape, or removes it, from its right-click menu', async () => {
    const curve = setup(twoBands())
    fireEvent.contextMenu(curve, dot(1000, 6))
    expect(screen.getByRole('menuitem', { name: '✓ Bell' })).toBeTruthy()
    await act(async () => { fireEvent.click(screen.getByRole('menuitem', { name: 'Notch' })) })
    expect(sets()).toEqual([[paramId(2, SHAPE), Number((5 / 6).toFixed(4)), 101]])

    fireEvent.contextMenu(curve, dot(1000, 6))
    await act(async () => { fireEvent.click(screen.getByRole('menuitem', { name: /Remove/ })) })
    expect(sets()[1]).toEqual([paramId(2, ON), 0, 102])
  })

  it('removes the selected band with Delete, and selects another', () => {
    const curve = setup(twoBands())
    fireEvent.pointerDown(curve, { ...dot(1000, 6), button: 0 })
    fireEvent.pointerUp(curve)
    fireEvent.keyDown(curve, { key: 'Delete' })
    expect(sets()).toEqual([[paramId(2, ON), 0, 101]])
    expect(screen.getByRole('region', { name: 'Band 1' })).toBeTruthy()
  })

  it('leaves the output to its own knob', () => {
    setup(twoBands())
    fireEvent.keyDown(screen.getByRole('slider', { name: 'Output' }), { key: 'End' })
    expect(sets()).toEqual([[OUTPUT, 1, undefined]])
  })
})
