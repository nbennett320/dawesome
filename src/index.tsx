import React from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import { Provider } from 'react-redux'
import { DndProvider } from 'react-dnd'
import { HTML5Backend } from 'react-dnd-html5-backend'
import { store } from './state/store'
import App from './App'
import './global.scss'
import './components/fourtrack/fourtrack.scss'
import { applyTheme, ResolvedTheme } from './theme'
import slateLight from '../engine/themes/test/slate-light.resolved.json'

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

const boot = async () => {
  let theme: ResolvedTheme | undefined
  try {
    const themes = await invoke<ResolvedTheme[]>('themes')
    // always slate-light for now; choosing by preferences comes in a later milestone
    theme = themes.find(t => t.id === 'slate-light')
    if(!theme) console.error('theme slate-light not found')
  } catch (e) {
    console.error('could not load themes', e)
  }
  applyTheme(theme ?? FALLBACK)

  root.render(
    <React.StrictMode>
      <Provider store={store}>
        <DndProvider backend={HTML5Backend}>
          <App />
        </DndProvider>
      </Provider>
    </React.StrictMode>,
  )
}

boot()
