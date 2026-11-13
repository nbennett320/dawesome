// keeps state.project.unsaved (the * in the window's title) in step with the
// engine, which is what knows whether the project has unsaved changes (see
// has_unsaved_changes in engine/src/project.rs)
//
// asking means the engine gathers the whole project as a save would, plugins'
// settings and all, on the main thread, so it isn't polled. it's asked a
// moment after the project's parts of the store stop changing (an edit, an
// undo, a sample that finished loading), and when the window gets the focus
// back (after turning a plugin's knobs in its own window)
import { invoke } from '@tauri-apps/api/core'
import type { AppDispatch, RootState } from './store'
import { setUnsaved } from './slices/projectSlice'

// how long the project has to stay still before asking: a fader dragged
// asks once, when it's let go
export const SETTLE_MS = 300

type Store = { dispatch: AppDispatch, getState: () => RootState, subscribe: (l: () => void) => () => void }

// the parts of the state that are saved with the project. any of them
// changing (by reference) may be a change. the rest of the store (the
// playhead, meters, the transport polled every half second) isn't
export const projectParts = (state: RootState): unknown[] => {
  const { playlist, sequencer, recording } = state
  return [
    playlist.playlistItems, playlist.clipSettings, playlist.clipWindows,
    playlist.tempo, playlist.metronomeEnabled, playlist.loopEnabled,
    playlist.snapEnabled, playlist.snapNote,
    state.mixer,
    sequencer.rows, sequencer.patterns, sequencer.folders,
    state.effects.racks,
    // a take lands in the playlist when recording stops
    recording.armed, recording.countIn, recording.recording,
    state.project.path, state.project.settled,
  ]
}

const sameParts = (a: unknown[], b: unknown[]) => a.every((part, i) => part === b[i])

// start watching. returns a function that stops
export const startUnsavedWatch = (store: Store, target: Pick<Window, 'addEventListener' | 'removeEventListener'> = window) => {
  let timer: ReturnType<typeof setTimeout> | null = null
  let asking = false
  let askAgain = false
  let stopped = false

  const ask = async () => {
    timer = null
    if(asking) {
      askAgain = true
      return
    }
    asking = true
    // a save, open or new project meanwhile already says nothing's unsaved,
    // and makes this answer out of date
    const settled = store.getState().project.settled
    try {
      const unsaved = await invoke<boolean>('project_has_unsaved_changes')
      const { project } = store.getState()
      if(!stopped && project.settled === settled && project.unsaved !== unsaved) {
        store.dispatch(setUnsaved(unsaved))
      }
    } catch {
      // no engine to ask (the UI in a plain browser): leave the title be
    } finally {
      asking = false
    }
    if(askAgain && !stopped) {
      askAgain = false
      askSoon()
    }
  }

  const askSoon = () => {
    if(stopped) return
    if(timer) clearTimeout(timer)
    timer = setTimeout(ask, SETTLE_MS)
  }

  let parts = projectParts(store.getState())
  const unsubscribe = store.subscribe(() => {
    const now = projectParts(store.getState())
    if(sameParts(parts, now)) return
    parts = now
    askSoon()
  })
  target.addEventListener('focus', askSoon)

  return () => {
    stopped = true
    if(timer) clearTimeout(timer)
    unsubscribe()
    target.removeEventListener('focus', askSoon)
  }
}
