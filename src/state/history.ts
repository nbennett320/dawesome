// Edit › Undo and Redo. the engine keeps the history (see
// engine/src/history.rs) and hands back the whole project as it is
// afterwards, which every slice then shows
import { invoke } from '@tauri-apps/api/core'
import type { AppDispatch, RootState } from './store'
import { Travelled } from '../types/project'
import { gestureActive } from './gesture'
import {
  dropQueuedClipSettings, flushClipSettings, reduceProjectView, refreshTrackCount,
} from './slices/playlistSlice'
import { clearPendingMixerSends, flushMixerSends } from './slices/mixerSlice'
import { clearPendingNoteSends, flushNoteSends, syncRun } from './slices/sequencerSlice'
import { dropHeldTrim } from './pendingTrim'

export type Way = 'undo' | 'redo'
// where the command came from: the Edit menu (or its shortcut), or the key
// press seen in the window. which of the two a shortcut arrives by depends
// on the system, and it can arrive by both
export type Source = 'menu' | 'key'

// the same press arriving a second time, by the other route, is dropped
const SAME_PRESS_MS = 150
let lastTravel: { way: Way, source: Source, at: number } | null = null

// whether a command is one already acted on, arriving again by the other route
export const samePress = (
  last: { way: Way, source: Source, at: number } | null,
  way: Way,
  source: Source,
  at: number,
): boolean =>
  !!last && last.way === way && last.source !== source && at - last.at < SAME_PRESS_MS

// one at a time, in the order they were asked for
let queue: Promise<unknown> = Promise.resolve()

// undo the last change to the project, or redo the last one undone.
// resolves to whether anything changed. nothing happens while the mouse is
// held (a drag is in progress), and the engine does nothing while recording
export const travel = (way: Way, source: Source = 'menu') => (
  dispatch: AppDispatch,
  getState: () => RootState,
): Promise<boolean> => {
  const now = Date.now()
  if(samePress(lastTravel, way, source, now)) return Promise.resolve(false)
  lastTravel = { way, source, at: now }

  if(gestureActive()) return Promise.resolve(false)
  // a trim held for the play cursor is forgotten now, so it can't land after
  // the undo, as a change of its own that ends redo (see pendingTrim.ts)
  dropHeldTrim()

  const run = async (): Promise<boolean> => {
    try {
      // a setting changed a moment ago may not have reached the engine yet:
      // it has to be there to be undone
      await Promise.all([flushClipSettings(getState), flushMixerSends(), flushNoteSends()])
      const travelled = await invoke<Travelled>(way, {})
      if(!travelled.changed) return false

      dropQueuedClipSettings()
      clearPendingMixerSends()
      clearPendingNoteSends()
      dispatch(reduceProjectView({ project: travelled.project, selected: travelled.selected }))
      // clips may have come back on, or gone from, the bottom tracks
      await refreshTrackCount()(dispatch)
      // the tempo may have changed, and with it where the playhead is
      await syncRun(dispatch)
      return true
    } catch(e) {
      // eslint-disable-next-line no-console
      console.error(`could not ${way}:`, e)
      return false
    }
  }

  const result = queue.then(run)
  queue = result
  return result
}

// for tests
export const resetTravel = (): void => {
  lastTravel = null
  queue = Promise.resolve()
}
