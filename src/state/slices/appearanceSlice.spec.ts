import type { Mock } from 'vitest'
import { configureStore } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import reducer, {
  changePreferences, defaultPreferences, hydrateAppearance, selectEffectiveThemeId, setSystemDark,
} from './appearanceSlice'
import { setThemes } from '../../theme/registry'
import { slateLightTheme } from '../../theme/fixture'
import type { Preferences, ResolvedTheme } from '../../theme/keys'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const mockInvoke = invoke as Mock

const theme = (id: string, appearance: 'light' | 'dark'): ResolvedTheme =>
  ({ ...slateLightTheme, id, name: id, appearance })
const all = [theme('slate-light', 'light'), theme('paper', 'light'), theme('charcoal', 'dark'), theme('ink', 'dark')]

const make = (preferences: Partial<Preferences> = {}, systemDark = false) => {
  setThemes(all)
  const store = configureStore({ reducer: { appearance: reducer } })
  store.dispatch(hydrateAppearance({
    preferences: { ...defaultPreferences, lightTheme: 'paper', darkTheme: 'ink', ...preferences },
    systemDark,
    themes: all.map(({ id, name, appearance }) => ({ id, name, appearance })),
  }))
  return store
}
const effective = (s: ReturnType<typeof make>) => selectEffectiveThemeId(s.getState() as never)

describe('selectEffectiveThemeId', () => {
  it.each([
    ['light', false, 'paper'],
    ['light', true, 'paper'],
    ['dark', false, 'ink'],
    ['dark', true, 'ink'],
    ['system', false, 'paper'],
    ['system', true, 'ink'],
  ] as const)('%s with systemDark %s is %s', (appearance, systemDark, id) => {
    expect(effective(make({ appearance }, systemDark))).toBe(id)
  })

  it('falls back to the default for an unknown id', () => {
    expect(effective(make({ appearance: 'light', lightTheme: 'nope' }))).toBe('slate-light')
    expect(effective(make({ appearance: 'dark', darkTheme: 'nope' }))).toBe('charcoal')
  })

  it('falls back to the default for an id of the wrong appearance', () => {
    expect(effective(make({ appearance: 'light', lightTheme: 'ink' }))).toBe('slate-light')
    expect(effective(make({ appearance: 'dark', darkTheme: 'paper' }))).toBe('charcoal')
  })
})

describe('hydrateAppearance', () => {
  it('replaces the state', () => {
    const store = make({ appearance: 'dark' }, true)
    expect(store.getState().appearance.preferences.appearance).toBe('dark')
    expect(store.getState().appearance.systemDark).toBe(true)
    expect(store.getState().appearance.themes).toHaveLength(4)
  })
})

describe('changePreferences', () => {
  beforeEach(() => {
    mockInvoke.mockReset()
    window.matchMedia = vi.fn().mockReturnValue({ matches: true }) as never
  })

  it('stores what the engine sanitized and rereads the system appearance', async () => {
    const store = make({ appearance: 'light' })
    const wanted = { ...defaultPreferences, lightTheme: 'paper', darkTheme: 'ink', appearance: 'dark' }
    mockInvoke.mockResolvedValue({ ...wanted, darkTheme: 'charcoal' })
    const done = (store.dispatch as never as (t: unknown) => Promise<void>)(changePreferences({ appearance: 'dark' }))
    expect(store.getState().appearance.preferences.appearance).toBe('dark')
    await done
    expect(mockInvoke).toHaveBeenCalledWith('set_preferences', { preferences: wanted })
    expect(store.getState().appearance.preferences.darkTheme).toBe('charcoal')
    expect(store.getState().appearance.systemDark).toBe(true)
  })

  it('keeps the change when the engine fails', async () => {
    const error = vi.spyOn(console, 'error').mockImplementation(() => {})
    const store = make({ appearance: 'light' })
    mockInvoke.mockRejectedValue('no engine')
    await (store.dispatch as never as (t: unknown) => Promise<void>)(changePreferences({ lightTheme: 'slate-light' }))
    expect(store.getState().appearance.preferences.lightTheme).toBe('slate-light')
    expect(error).toHaveBeenCalled()
    error.mockRestore()
  })

  it('setSystemDark sets the flag', () => {
    expect(reducer(undefined, setSystemDark(true)).systemDark).toBe(true)
  })
})
