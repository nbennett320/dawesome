// the clip clipboard commands, run the same way wherever they came from: a
// key, the Edit menu, the system's copy, cut and paste events, or a context
// menu
import type { Dispatch } from '@reduxjs/toolkit'
import type { RootState } from './store'
import { copyClips, cutClips, deepCopyClips, joinClips, pasteClips, repeatClips } from './slices/playlistSlice'

export type ClipCommand = 'copy' | 'cut' | 'paste' | 'repeat' | 'select-all' | 'join' | 'deep-copy'

export interface ClipTarget {
  // the selected clips
  selected: number[]
  // every clip
  all: number[]
  // where a paste goes: the playhead, snapped by the caller. null if unknown
  pasteBeat: number | null
  // whether the Playlist is the focused view
  focused: boolean
}

// run a command on the clips in `target`. nothing happens unless the
// Playlist is focused, nor to copy, cut or repeat with nothing selected.
// `select` makes the playlist select clips, and `setBusy` shows the wait
// cursor while a slow command writes files
export const clipCommand = (
  command: ClipCommand,
  target: ClipTarget,
  select: (ids: number[]) => void,
  setBusy: (busy: boolean) => void = () => {},
) => async (dispatch: Dispatch, getState: () => RootState): Promise<unknown> => {
  if(!target.focused) return undefined
  const { selected, all, pasteBeat } = target

  switch(command) {
    case 'copy': return selected.length ? copyClips(selected)(dispatch) : undefined
    case 'cut': return selected.length ? cutClips(selected)(dispatch) : undefined
    case 'paste': return pasteBeat === null ? undefined : pasteClips(pasteBeat, null)(dispatch)
    case 'repeat': return selected.length ? repeatClips(selected)(dispatch) : undefined
    case 'join': return selected.length >= 2 ? joinClips(selected, setBusy)(dispatch, getState) : undefined
    case 'deep-copy': return selected.length ? deepCopyClips(selected, setBusy)(dispatch, getState) : undefined
    case 'select-all':
      select(all)
      return undefined
    default: return undefined
  }
}

// where a command came from. a shortcut can reach the app by more than one
export type ClipSource = 'key' | 'menu' | 'clipboard-event'

const SAME_COMMAND_MS = 150

// whether a command is one already acted on, arriving again by another route
export const sameCommand = (
  last: { command: ClipCommand, source: ClipSource, at: number } | null,
  command: ClipCommand,
  source: ClipSource,
  at: number,
): boolean =>
  !!last && last.command === command && last.source !== source && at - last.at < SAME_COMMAND_MS

// the Playlist registers how it runs a command, so the Edit menu's items
// reach it without the menu's code knowing about the canvas
let runner: ((command: ClipCommand, source: 'menu') => void) | null = null

export const setClipCommandRunner = (run: ((command: ClipCommand, source: 'menu') => void) | null) => {
  runner = run
}

export const runClipCommandFromMenu = (command: ClipCommand): void => {
  runner?.(command, 'menu')
}
