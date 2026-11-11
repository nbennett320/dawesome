import { invoke } from '@tauri-apps/api/core'
import type { AppDispatch, RootState } from '../state/store'
import { selectEffectiveThemeId, setSystemDark } from '../state/slices/appearanceSlice'
import { applyTheme } from './apply'
import { allThemes, getTheme } from './registry'

const QUERY = '(prefers-color-scheme: dark)'

// keeps the applied theme in step with the store, and the store with the OS.
// returns a function that stops
export const startThemeSync = (store: { dispatch: AppDispatch, getState: () => RootState, subscribe: (l: () => void) => () => void }) => {
  let appliedId: string | null = null

  const sync = () => {
    const id = selectEffectiveThemeId(store.getState())
    if(id === appliedId) return
    // with only the bundled fallback there may be nothing of the right appearance
    const theme = getTheme(id) ?? allThemes()[0]
    if(!theme) return
    appliedId = id
    applyTheme(theme)
    // the native window shows through while resizing and overscrolling, so it
    // matches the page's backdrop (global.scss; BACKDROP in engine/src/app/window.rs)
    invoke('set_window_background', { color: theme.colors.panel })
      // eslint-disable-next-line no-console
      .catch(e => console.error('could not set the window background', e))
  }

  const unsubscribe = store.subscribe(sync)
  sync()

  const media = typeof window.matchMedia === 'function' ? window.matchMedia(QUERY) : null
  const onMedia = (ev: { matches: boolean }) => store.dispatch(setSystemDark(ev.matches))
  media?.addEventListener('change', onMedia)

  return () => {
    unsubscribe()
    media?.removeEventListener('change', onMedia)
  }
}
