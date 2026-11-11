import { channels } from './color'
import type { CoreKey, ResolvedTheme, ThemeKey } from './keys'

let current: ResolvedTheme | null = null
const listeners = new Set<() => void>()

const kebab = (s: string) => s.replace(/[A-Z]/g, c => `-${c.toLowerCase()}`)

export const currentTheme = (): ResolvedTheme | null => current

export const onThemeChange = (listener: () => void): (() => void) => {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}

export const applyTheme = (theme: ResolvedTheme) => {
  const root = document.documentElement
  ;(Object.entries(theme.colors) as [ThemeKey, string][]).forEach(([key, value]) => {
    if(key.startsWith('fourtrack.')) {
      root.style.setProperty(`--ft-${kebab(key.slice('fourtrack.'.length))}`, value)
    } else if(!key.includes('.')) {
      // core colors are opaque: channels carry no alpha, so it is dropped
      root.style.setProperty(`--c-${kebab(key)}`, channels(value))
    }
  })
  root.style.colorScheme = theme.appearance
  root.dataset.theme = theme.id
  root.dataset.appearance = theme.appearance
  current = theme
  listeners.forEach(l => l())
}

/** a core color for inline DOM styles; follows theme changes without re-rendering */
export const cssVar = (key: CoreKey, alpha?: number): string =>
  alpha === undefined ? `rgb(var(--c-${kebab(key)}))` : `rgb(var(--c-${kebab(key)}) / ${alpha})`
