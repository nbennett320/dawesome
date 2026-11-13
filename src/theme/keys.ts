// theme key types, derived from the engine's base.json (no codegen)
import base from '../../engine/themes/base.json'

type Base = typeof base

export type CoreKey = keyof Base['colors']
export type SectionName = Exclude<keyof Base, 'colors'>
export type SectionKey<S extends SectionName> = keyof Base[S]

export type ThemeKey =
  | CoreKey
  | { [S in SectionName]: `${S}.${SectionKey<S> & string}` }[SectionName]

export type Appearance = 'light' | 'dark'

export interface ResolvedTheme {
  id: string
  name: string
  appearance: Appearance
  colors: Record<ThemeKey, string>
}

export interface Preferences {
  appearance: 'system' | Appearance
  lightTheme: string
  darkTheme: string
  // where a plugin's own interface opens: a floating window, or a tab
  pluginView: 'window' | 'tab'
  // how the Drum Sequencer's step keys look (see StepKeys in preferences.rs)
  stepKeys: StepKeys
}

// coloured caps (TR-808), plain caps over a coloured strip (TR-909), or
// pads that light up (TR-8S)
export type StepKeys = 'tr808' | 'tr909' | 'tr8s'
