import type { Mock } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { fireEvent, render, screen, within } from '@testing-library/react'
import playlistReducer from '../../../state/slices/playlistSlice'
import { applyTheme } from '../../../theme/apply'
import { slateLightTheme } from '../../../theme/fixture'
import { normalizedOf, ParamWire } from '../../../types/nativeEffect'
import { param } from '../../../types/nativeEffect.fixture'
import type { EffectParamsApi } from '../useEffectParams'
import DelayPanel from './DelayPanel'
import { FEEDBACK, MODE, NOTE, settingsOf, spanOf, TIME } from './echoes'
import { timeToX } from './geometry'

vi.mock('../../../state/gesture', () => ({
  currentGesture: () => null,
  newGesture: () => 1,
}))

// the engine's (engine/crates/dsp/src/effects/delay.rs)
const NOTES = [
  '1/32 T', '1/32', '1/16 T', '1/32 D', '1/16', '1/8 T', '1/16 D', '1/8', '1/4 T',
  '1/8 D', '1/4', '1/2 T', '1/4 D', '1/2', '1/1 T', '1/2 D', '1/1', '1/1 D',
]
const area = { width: 800, height: 400 }

interface Values { sync?: boolean, note?: string, timeMs?: number, offset?: number, pingPong?: boolean, feedback?: number, mix?: number }

// the Delay's parameters, as the engine describes them
const delayParams = (v: Values = {}): ParamWire[] => {
  const params: ParamWire[] = []
  const add = (change: Partial<ParamWire>, plain: number) => {
    const id = params.length
    const p = param(id, change)
    params.push({ ...p, value: { id, value: plain, normalized: normalizedOf(p, plain), text: `${plain}` } })
  }
  add({ name: 'Sync', group: 'Time', stepCount: 1 }, v.sync === false ? 0 : 1)
  add({ name: 'Note', group: 'Time', max: 17, stepCount: 17, labels: NOTES }, NOTES.indexOf(v.note ?? '1/4 D'))
  add({ name: 'Time', group: 'Time', unit: 'ms', min: 1, max: 2000, taper: 'log' }, v.timeMs ?? 375)
  add({ name: 'Offset', group: 'Time', unit: '%', min: -50, max: 50 }, v.offset ?? 0)
  add({ name: 'Mode', group: 'Echoes', max: 1, stepCount: 1, labels: ['Stereo', 'Ping-pong'] }, v.pingPong ? 1 : 0)
  add({ name: 'Feedback', group: 'Echoes', unit: '%', min: 0, max: 95 }, (v.feedback ?? 0.4) * 100)
  add({ name: 'Low cut', group: 'Tone', unit: 'Hz', min: 20, max: 2000, taper: 'log' }, 80)
  add({ name: 'High cut', group: 'Tone', unit: 'Hz', min: 1000, max: 20000, taper: 'log' }, 8000)
  add({ name: 'Mix', group: 'Mix', unit: '%', min: 0, max: 100 }, v.mix ?? 30)
  return params
}

let set: Mock<EffectParamsApi['set']>

const setup = (params: ParamWire[]) => {
  set = vi.fn<EffectParamsApi['set']>()
  const api: EffectParamsApi = { status: { kind: 'ready', effect: 'delay' }, params, set, setText: vi.fn() }
  const store = configureStore({ reducer: { playlist: playlistReducer } })
  render(<Provider store={store}><DelayPanel api={api} slotId={3} active /></Provider>)
  return screen.getByLabelText(/^Delay echoes/)
}

// where an echo `seconds` after the sound is, at these settings (tempo 120)
const echoAt = (params: ParamWire[], seconds: number) =>
  timeToX(seconds, spanOf(settingsOf(params)!, 120), area)

// the calls to set, as [id, normalized]
const sets = () => set.mock.calls.map(([id, n]) => [id, Number(n.toFixed(4))])

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

describe('the Delay’s panel', () => {
  it('shows the note value while synced, and the time while not', () => {
    setup(delayParams())
    expect(screen.getByRole('combobox', { name: 'Time Note' })).toBeTruthy()
    expect(screen.queryByRole('slider', { name: 'Time' })).toBeNull()
    for(const name of ['Time Offset', 'Echoes Feedback', 'Tone Low cut', 'Tone High cut', 'Mix']) {
      expect(screen.getByRole('slider', { name })).toBeTruthy()
    }
  })

  it('shows the time knob with sync off', () => {
    setup(delayParams({ sync: false }))
    expect(screen.getByRole('slider', { name: 'Time' })).toBeTruthy()
    expect(screen.queryByRole('combobox', { name: 'Time Note' })).toBeNull()
  })

  it('switches mode with two keys, the one that’s on held down', () => {
    setup(delayParams())
    const keys = within(screen.getByRole('group', { name: 'Mode' }))
    expect(keys.getByRole('button', { name: 'Stereo' }).getAttribute('aria-pressed')).toBe('true')
    expect(keys.getByRole('button', { name: 'Ping-pong' }).getAttribute('aria-pressed')).toBe('false')
    fireEvent.click(keys.getByRole('button', { name: 'Ping-pong' }))
    expect(sets()).toEqual([[MODE, 1]])
  })

  it('drags the first echo to the nearest note value, synced', () => {
    const params = delayParams()
    const view = setup(params)
    // 1/4 D at 120 bpm is 750 ms; dragged to a second: a half note
    fireEvent.pointerDown(view, { clientX: echoAt(params, 0.75), clientY: 100, button: 0 })
    fireEvent.pointerMove(view, { clientX: echoAt(params, 1), clientY: 100 })
    fireEvent.pointerUp(view)
    expect(sets()).toEqual([[NOTE, Number((NOTES.indexOf('1/2') / 17).toFixed(4))]])
  })

  it('drags the time freely with sync off, and finely with shift', () => {
    const params = delayParams({ sync: false, timeMs: 250 })
    const view = setup(params)
    const time = params[TIME]
    fireEvent.pointerDown(view, { clientX: echoAt(params, 0.25), clientY: 100, button: 0 })
    fireEvent.pointerMove(view, { clientX: echoAt(params, 0.5), clientY: 100 })
    fireEvent.pointerUp(view)
    expect(sets()).toEqual([[TIME, Number(normalizedOf(time, 500).toFixed(4))]])

    set.mockClear()
    fireEvent.pointerDown(view, { clientX: echoAt(params, 0.25), clientY: 100, button: 0, shiftKey: true })
    fireEvent.pointerMove(view, { clientX: echoAt(params, 0.65), clientY: 100, shiftKey: true })
    // a quarter of the way: 250 + 100 ms
    expect(sets()).toEqual([[TIME, Number(normalizedOf(time, 350).toFixed(4))]])
  })

  it('drags the longer side’s first echo to where its time puts it, offset counted', () => {
    const params = delayParams({ sync: false, timeMs: 200, offset: 50 })
    const view = setup(params)
    // the right side is 1.5 times as long: its first echo is at 300 ms
    fireEvent.pointerDown(view, { clientX: echoAt(params, 0.3), clientY: 300, button: 0 })
    fireEvent.pointerMove(view, { clientX: echoAt(params, 0.6), clientY: 300 })
    expect(sets()).toEqual([[TIME, Number(normalizedOf(params[TIME], 400).toFixed(4))]])
  })

  it('drags the feedback up and down', () => {
    const params = delayParams({ sync: false, timeMs: 250, feedback: 0.38 })
    const view = setup(params)
    const from = { clientX: echoAt(params, 0.25), clientY: 150 }
    fireEvent.pointerDown(view, { ...from, button: 0 })
    // a quarter of the height up: half the range more
    fireEvent.pointerMove(view, { clientX: from.clientX, clientY: 50 })
    const feedback = set.mock.calls.filter(([id]) => id === FEEDBACK).map(([, n]) => n)
    expect(feedback).toHaveLength(1)
    expect(feedback[0]).toBeCloseTo(0.4 + 0.5)
  })

  it('takes nothing away from the first echoes', () => {
    const params = delayParams({ sync: false, timeMs: 250 })
    const view = setup(params)
    fireEvent.pointerDown(view, { clientX: echoAt(params, 0.5), clientY: 100, button: 0 })
    fireEvent.pointerMove(view, { clientX: echoAt(params, 0.9), clientY: 10 })
    expect(set).not.toHaveBeenCalled()
  })
})
