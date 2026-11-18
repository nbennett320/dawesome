import type { Mock } from 'vitest'
import { act, renderHook } from '@testing-library/react'
import { invoke } from '@tauri-apps/api/core'
import { useEffectReadout } from './useEffectReadout'
import { POLL_MS } from './spectrum'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

const polls = () => mockInvoke.mock.calls.filter(([name]) => name === 'get_effect_readout').length

let written = 0
beforeEach(() => {
  vi.resetAllMocks()
  vi.useFakeTimers()
  written = 0
  mockInvoke.mockImplementation(async () => ({ written, rate: 48000, slots: [0.25, 3] }))
})

afterEach(() => {
  vi.useRealTimers()
})

describe('polling an effect’s readout', () => {
  it('asks while active, and says whether it’s being run', async () => {
    const onReading = vi.fn()
    const { rerender } = renderHook(({ active }) => useEffectReadout(3, active, onReading), { initialProps: { active: true } })
    await act(async () => {})
    expect(mockInvoke).toHaveBeenCalledWith('get_effect_readout', { slotId: 3 })
    // (the first can't tell)
    expect(onReading.mock.lastCall?.[0]).toMatchObject({ readout: { slots: [0.25, 3] }, running: false })

    written = 512
    await act(async () => { vi.advanceTimersByTime(POLL_MS) })
    expect(onReading.mock.lastCall?.[0].running).toBe(true)
    // nothing more run: it's resting
    await act(async () => { vi.advanceTimersByTime(POLL_MS) })
    expect(onReading.mock.lastCall?.[0].running).toBe(false)

    rerender({ active: false })
    const before = polls()
    await act(async () => { vi.advanceTimersByTime(POLL_MS * 10) })
    expect(polls()).toBe(before)
  })
})
