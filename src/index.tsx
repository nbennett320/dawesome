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

const container = document.getElementById('root') as Element
const root = createRoot(container)

const boot = async () => {
  try {
    const themes = await invoke<ResolvedTheme[]>('themes')
    // always slate-light for now; choosing by preferences comes in a later milestone
    const theme = themes.find(t => t.id === 'slate-light')
    if(theme) applyTheme(theme)
    else console.error('theme slate-light not found')
  } catch (e) {
    console.error('could not load themes', e)
  }

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
