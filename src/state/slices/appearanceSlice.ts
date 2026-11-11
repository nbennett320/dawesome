import { createSlice, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { AppDispatch, RootState } from '../store'
import type { Appearance, Preferences } from '../../theme/keys'
import { defaultThemeId, getTheme } from '../../theme/registry'

export interface ThemeSummary {
  id: string
  name: string
  appearance: Appearance
}

export interface AppearanceState {
  preferences: Preferences
  // whether the OS (or the window, when pinned) is dark
  systemDark: boolean
  themes: ThemeSummary[]
}

export const defaultPreferences: Preferences = {
  appearance: 'system',
  lightTheme: defaultThemeId('light'),
  darkTheme: defaultThemeId('dark'),
  pluginView: 'window',
}

const initialState: AppearanceState = {
  preferences: defaultPreferences,
  systemDark: false,
  themes: [],
}

export const appearanceSlice = createSlice({
  name: 'appearanceSlice',
  initialState,
  reducers: {
    hydrateAppearance: (_state, action: PayloadAction<AppearanceState>) => action.payload,
    setPreferences: (state, action: PayloadAction<Preferences>) => { state.preferences = action.payload },
    setSystemDark: (state, action: PayloadAction<boolean>) => { state.systemDark = action.payload },
  },
})

export const { hydrateAppearance, setPreferences, setSystemDark } = appearanceSlice.actions

export const selectAppearance = (state: RootState) => state.appearance

// the theme in use: the chosen one for the appearance in force, or that
// appearance's default when the id is unknown or belongs to the other one
export const selectEffectiveThemeId = (state: RootState): string => {
  const { preferences, systemDark } = state.appearance
  const dark = preferences.appearance === 'system' ? systemDark : preferences.appearance === 'dark'
  const appearance: Appearance = dark ? 'dark' : 'light'
  const id = dark ? preferences.darkTheme : preferences.lightTheme
  return getTheme(id)?.appearance === appearance ? id : defaultThemeId(appearance)
}

const prefersDark = () =>
  typeof window.matchMedia === 'function' && window.matchMedia('(prefers-color-scheme: dark)').matches

// applies a change at once, then has the engine sanitize and save it. where it
// can't (a plain browser) the change stays
export const changePreferences = (partial: Partial<Preferences>) =>
  async (dispatch: AppDispatch, getState: () => RootState) => {
    const wanted = { ...getState().appearance.preferences, ...partial }
    dispatch(setPreferences(wanted))
    try {
      const saved = await invoke<Preferences>('set_preferences', { preferences: wanted })
      dispatch(setPreferences(saved))
    } catch (e) {
      // eslint-disable-next-line no-console
      console.error('could not save preferences', e)
    }
    // a pinned window reports its own appearance, so System has to look again
    dispatch(setSystemDark(prefersDark()))
  }

export default appearanceSlice.reducer
