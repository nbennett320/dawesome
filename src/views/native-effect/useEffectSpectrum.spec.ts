import type { Mock } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { useEffectSpectrum } from './useEffectSpectrum'
import { POLL_MS } from './spectrum'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

const polls = () => mockInvoke.mock.calls.filter(([name]) => name === 'get_effect_spectrum').length

let hidden = false
beforeAll(() => {
  Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => (hidden ? 'hidden' : 'visible') })
})
afterAll(() => {
  delete (document as unknown as Record<string, unknown>).visibilityState
})

beforeEach(() => {
  vi.resetAllMocks()
  vi.useFakeTimers()
  hidden = false
  let written = 0
  mockInvoke.mockImplementation(async () => {
    written += 512
    return { written, rate: 48000, levels: [-20, -60] }
  })
})

afterEach(() => {
  vi.useRealTimers()
})

describe('polling an effect’s spectrum', () => {
  it('asks while active, handing each picture over, and stops when it isn’t', async () => {
    const onFrame = vi.fn()
    const { rerender, unmount } = renderHook(({ active }) => useEffectSpectrum(7, active, onFrame), { initialProps: { active: true } })
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('get_effect_spectrum', { slotId: 7 })
    expect(onFrame).toHaveBeenLastCalledWith({ levels: [-20, -60], written: 512 })

    // (one at a time: each answered before the next)
    for(let i = 0; i < 3; i += 1) await act(async () => { vi.advanceTimersByTime(POLL_MS) })
    expect(polls()).toBe(4)

    rerender({ active: false })
    await act(async () => { vi.advanceTimersByTime(POLL_MS * 10) })
    expect(polls()).toBe(4)

    rerender({ active: true })
    await act(async () => {})
    expect(polls()).toBe(5)
    unmount()
    await act(async () => { vi.advanceTimersByTime(POLL_MS * 10) })
    expect(polls()).toBe(5)
  })

  it('stops while the window can’t be seen', async () => {
    renderHook(() => useEffectSpectrum(7, true, vi.fn()))
    await act(async () => {})
    hidden = true
    act(() => { document.dispatchEvent(new Event('visibilitychange')) })
    const before = polls()
    await act(async () => { vi.advanceTimersByTime(POLL_MS * 10) })
    expect(polls()).toBe(before)
  })

  it('doesn’t ask again while a poll is still unanswered', async () => {
    mockInvoke.mockImplementation(() => new Promise(() => {}))
    renderHook(() => useEffectSpectrum(7, true, vi.fn()))
    await act(async () => { vi.advanceTimersByTime(POLL_MS * 5) })
    expect(polls()).toBe(1)
  })

  it('carries on quietly after a poll fails', async () => {
    mockInvoke.mockRejectedValueOnce('gone')
    const onFrame = vi.fn()
    renderHook(() => useEffectSpectrum(7, true, onFrame))
    await act(async () => {})
    expect(onFrame).not.toHaveBeenCalled()
    await act(async () => { vi.advanceTimersByTime(POLL_MS) })
    expect(onFrame).toHaveBeenCalledTimes(1)
  })
})
