import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import appearanceReducer, { defaultPreferences, hydrateAppearance, setPreferences } from '../state/slices/appearanceSlice'
import { startThemeSync } from './sync'
import { currentTheme } from './apply'
import { setThemes } from './registry'
import { slateLightTheme } from './fixture'
import type { ResolvedTheme } from './keys'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

const dark: ResolvedTheme = {
  ...slateLightTheme, id: 'charcoal', name: 'Charcoal', appearance: 'dark',
  colors: { ...slateLightTheme.colors, panel: '#101010' },
}
const all = [slateLightTheme, dark]

describe('startThemeSync', () => {
  let changeMedia: (matches: boolean) => void
  const removeListener = vi.fn()

  const setup = () => {
    setThemes(all)
    const store = configureStore({ reducer: { appearance: appearanceReducer } })
    store.dispatch(hydrateAppearance({
      preferences: defaultPreferences,
      systemDark: false,
      themes: all.map(({ id, name, appearance }) => ({ id, name, appearance })),
    }))
    return store
  }

  beforeEach(() => {
    mockInvoke.mockReset().mockResolvedValue(undefined)
    removeListener.mockReset()
    window.matchMedia = vi.fn().mockReturnValue({
      matches: false,
      addEventListener: (_: string, l: (ev: { matches: boolean }) => void) => { changeMedia = matches => l({ matches }) },
      removeEventListener: removeListener,
    }) as never
  })

  it('applies now, and the window background with it', () => {
    const stop = startThemeSync(setup() as never)
    expect(currentTheme()).toBe(slateLightTheme)
    expect(mockInvoke).toHaveBeenCalledWith('set_window_background', { color: slateLightTheme.colors.panel })
    stop()
  })

  it('applies on a change of theme but not of anything else', () => {
    const store = setup()
    const stop = startThemeSync(store as never)
    mockInvoke.mockClear()
    store.dispatch(setPreferences({ ...defaultPreferences, darkTheme: 'charcoal' }))
    expect(mockInvoke).not.toHaveBeenCalled()
    store.dispatch(setPreferences({ ...defaultPreferences, appearance: 'dark' }))
    expect(currentTheme()).toBe(dark)
    expect(mockInvoke).toHaveBeenCalledTimes(1)
    expect(mockInvoke).toHaveBeenCalledWith('set_window_background', { color: '#101010' })
    stop()
  })

  it('follows the system appearance', () => {
    const store = setup()
    const stop = startThemeSync(store as never)
    changeMedia(true)
    expect(store.getState().appearance.systemDark).toBe(true)
    expect(currentTheme()).toBe(dark)
    stop()
    expect(removeListener).toHaveBeenCalled()
  })

  it('logs a failed background', async () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {})
    mockInvoke.mockRejectedValue('no')
    const stop = startThemeSync(setup() as never)
    await new Promise<void>(r => { setTimeout(r, 0) })
    expect(error).toHaveBeenCalled()
    error.mockRestore()
    stop()
  })
})
