import React from 'react'
import { createRoot } from 'react-dom/client'
import { Provider } from 'react-redux'
import { DndProvider } from 'react-dnd'
import { HTML5Backend } from 'react-dnd-html5-backend'
import { store } from './state/store'
import App from './App'
import './global.scss'
import './components/fourtrack/fourtrack.scss'
import { Preferences, ResolvedTheme, setThemes, startThemeSync } from './theme'
import { defaultPreferences, hydrateAppearance } from './state/slices/appearanceSlice'
import slateLight from '../engine/themes/test/slate-light.resolved.json'

declare global {
  interface Window {
    // set by the engine before any script runs; absent in a plain browser
    __DAWESOME_APPEARANCE__?: { preferences: Preferences, themes: ResolvedTheme[] }
  }
}

const container = document.getElementById('root') as Element
const root = createRoot(container)

// Slate Light as the engine resolves it (a Rust test keeps the two equal), for
// when the engine can't be asked, e.g. the UI in a plain browser. the app
// can't draw without a theme
const FALLBACK: ResolvedTheme = {
  id: 'slate-light',
  name: 'Slate Light',
  appearance: 'light',
  colors: slateLight as ResolvedTheme['colors'],
}

// eslint-disable-next-line no-underscore-dangle
const injected = window.__DAWESOME_APPEARANCE__
const themes = injected?.themes?.length ? injected.themes : [FALLBACK]
const preferences = injected?.preferences ?? defaultPreferences

setThemes(themes)
store.dispatch(hydrateAppearance({
  preferences,
  systemDark: typeof window.matchMedia === 'function'
    && window.matchMedia('(prefers-color-scheme: dark)').matches,
  themes: themes.map(({ id, name, appearance }) => ({ id, name, appearance })),
}))
// applies the theme, and keeps it applied
startThemeSync(store)

root.render(
  <React.StrictMode>
    <Provider store={store}>
      <DndProvider backend={HTML5Backend}>
        <App />
      </DndProvider>
    </Provider>
  </React.StrictMode>,
)
