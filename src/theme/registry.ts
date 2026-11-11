import type { Appearance, ResolvedTheme } from './keys'

export const DEFAULT_LIGHT_THEME = 'slate-light'
export const DEFAULT_DARK_THEME = 'charcoal'

export const defaultThemeId = (appearance: Appearance) =>
  appearance === 'light' ? DEFAULT_LIGHT_THEME : DEFAULT_DARK_THEME

// the colors stay here, out of redux: they're big and never change while running
let themes: ResolvedTheme[] = []

export const setThemes = (all: ResolvedTheme[]) => { themes = all }

export const allThemes = (): ResolvedTheme[] => themes

export const getTheme = (id: string): ResolvedTheme | undefined => themes.find(t => t.id === id)

export const themesFor = (appearance: Appearance): ResolvedTheme[] =>
  themes.filter(t => t.appearance === appearance)
