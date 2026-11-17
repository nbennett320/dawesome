// the clip clipboard commands, run the same way wherever they came from: a
// key, the Edit menu, the system's copy, cut and paste events, or a context
// menu
import type { Dispatch } from '@reduxjs/toolkit'
import type { RootState } from './store'
import {
  copyClips, copySpan, cutClips, cutSpan, deepCopyClips, joinClips, pasteAt, repeatClips, repeatSpan, splitClipsAt,
} from './slices/playlistSlice'
import type { Span } from '../types/project'

export type ClipCommand = 'copy' | 'cut' | 'paste' | 'repeat' | 'select-all' | 'join' | 'deep-copy' | 'blade'

export interface ClipTarget {
  // the selected clips
  selected: number[]
  // every clip
  all: number[]
  // where a paste goes: the playhead, snapped by the caller. null if unknown
  pasteBeat: number | null
  // whether the Playlist is the focused view
  focused: boolean
  // what Blade (⌘B) cuts: these clips, at these beats. null for nothing
  blade: { ids: number[], beats: number[] } | null
  // the time selection's span, if it covers any time: copy, cut and repeat
  // act on it rather than on the selected clips
  span: Span | null
}

// run a command on the clips in `target`. nothing happens unless the
// Playlist is focused, nor to copy, cut or repeat with nothing selected.
// `select` makes the playlist select clips, `setBusy` shows the wait cursor
// while a slow command writes files, and `selectSpan` makes the playlist
// select the span a paste or a repeat of a span filled
export const clipCommand = (
  command: ClipCommand,
  target: ClipTarget,
  select: (ids: number[]) => void,
  setBusy: (busy: boolean) => void = () => {},
  selectSpan: (span: Span) => void = () => {},
) => async (dispatch: Dispatch, getState: () => RootState): Promise<unknown> => {
  if(!target.focused) return undefined
  const { selected, all, pasteBeat, span } = target

  switch(command) {
    case 'copy':
      if(span) return copySpan(span)(dispatch)
      return selected.length ? copyClips(selected)(dispatch) : undefined
    case 'cut':
      if(span) return cutSpan(span)(dispatch)
      return selected.length ? cutClips(selected)(dispatch) : undefined
    // (a span on the clipboard goes on the current track)
    case 'paste': return pasteBeat === null ? undefined : pasteAt(pasteBeat, null, selectSpan)(dispatch, getState)
    case 'repeat':
      if(span) {
        const edit = await repeatSpan(span)(dispatch)
        if(edit?.span) selectSpan(edit.span)
        return edit
      }
      return selected.length ? repeatClips(selected)(dispatch) : undefined
    case 'join': return selected.length >= 2 ? joinClips(selected, setBusy)(dispatch, getState) : undefined
    case 'deep-copy': return selected.length ? deepCopyClips(selected, setBusy)(dispatch, getState) : undefined
    case 'blade': return target.blade ? splitClipsAt(target.blade.ids, target.blade.beats)(dispatch) : undefined
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
