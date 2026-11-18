import { invoke } from '@tauri-apps/api/core'
import type { Dispatch } from '@reduxjs/toolkit'
import { DroppedFile, SkippedFile, fileName } from './sampleDrag'
import {
  addPlaylistItem, playlistSlice, refreshTrackCount, selectClips, withWaitCursor,
} from './slices/playlistSlice'
import type { PlaylistItem, PlaylistItemPixelOffset } from '../types/playlist'
import type { ClipInfo } from '../types/clip'

// files dragged in from outside the app (see events/fileDrop.ts): copied
// into the project's clips (import_files), then placed in the playlist
// (place_files) or the Pattern Sequencer. see engine/src/daw/file_drop.rs

// a file brought into the clips: where it came from, and the file to play
export interface ImportedFile {
  source: string
  path: string
  // not copied: the project already had it
  reused: boolean
}

export interface Imported {
  files: ImportedFile[]
  skipped: SkippedFile[]
}

// more than this many files dropped on the playlist asks first whether to
// put them all on tracks
export const ASK_ABOVE = 16

export const PLACE_ALL = 'Place all'
export const ONLY_COPY = 'Only copy to Clips'

// a sentence naming what was left out, and why
export const leftOutSentence = (skipped: SkippedFile[], most = 6): string => {
  const named = skipped.slice(0, most).map(({ path, reason }) => `${fileName(path)} (${reason})`)
  const more = skipped.length - named.length
  return `Left out: ${named.join(', ')}${more > 0 ? `, and ${more} more` : ''}.`
}

const notice = async (dispatch: Dispatch, kind: 'info' | 'error', message: string) => {
  const { setProjectNotice } = await import('./slices/projectSlice')
  dispatch(setProjectNotice({ kind, message }))
}

// whether to place a big drop on tracks, only copy it, or neither
const askAboutMany = async (count: number): Promise<'place' | 'copy' | null> => {
  const { message } = await import('@tauri-apps/plugin-dialog')
  const answer = await message(
    `Place all ${count} files on tracks, one per track, or only copy them to Clips?`,
    { title: `${count} files`, kind: 'info', buttons: { yes: PLACE_ALL, no: ONLY_COPY, cancel: 'Cancel' } },
  )
  // (custom buttons answer with their label; the default ones by name)
  if(answer === PLACE_ALL || answer === 'Yes') return 'place'
  if(answer === ONLY_COPY || answer === 'No') return 'copy'
  return null
}

// copy files into the clips (into `into`, a folder of them, '' for the top),
// say what was left out, and have the Clips page list them. null if nothing
// could be brought in
export const importFiles = async (
  dispatch: Dispatch,
  files: DroppedFile[],
  skipped: SkippedFile[],
  into = '',
  setBusy: (busy: boolean) => void = () => {},
): Promise<Imported | null> => {
  let imported: Imported
  try {
    imported = files.length
      ? await withWaitCursor(invoke<Imported>('import_files', { files, into }), setBusy)
      : { files: [], skipped: [] }
  } catch(e) {
    await notice(dispatch, 'error', String(e))
    return null
  }

  const left = [...skipped, ...imported.skipped]
  if(left.length) await notice(dispatch, 'error', leftOutSentence(left))
  else if(!imported.files.length) await notice(dispatch, 'info', 'Nothing dragged in can be played.')

  if(imported.files.some(file => !file.reused)) {
    // loaded when needed: the recording slice imports the playlist slice
    const { takesChanged } = await import('./slices/recordingSlice')
    dispatch(takesChanged())
  }
  return imported.files.length ? imported : null
}

// files dropped on the playlist: copied into the clips, then a clip of each,
// one per track from `trackNumber` down, all at `dropBeat` (snapped). more
// than ASK_ABOVE asks first. returns the ids of the clips placed
export const dropFilesOnPlaylist = (
  files: DroppedFile[],
  skipped: SkippedFile[],
  trackNumber: number,
  dropBeat: number,
  pixelOffset: Omit<PlaylistItemPixelOffset, 'xOffset' | 'yOffset'>,
  setBusy: (busy: boolean) => void = () => {},
) => async (dispatch: Dispatch): Promise<number[]> => {
  const choice = files.length > ASK_ABOVE ? await askAboutMany(files.length) : 'place'
  if(!choice) return []

  const imported = await importFiles(dispatch, files, skipped, '', setBusy)
  if(!imported || choice === 'copy') return []

  let placed: { id: number, startBeat: number }[]
  try {
    placed = await withWaitCursor(invoke<{ id: number, startBeat: number }[]>('place_files', {
      paths: imported.files.map(file => file.path),
      trackNumber,
      dropBeat,
    }), setBusy)
  } catch(e) {
    await notice(dispatch, 'error', String(e))
    return []
  }

  const offset = { ...pixelOffset, xOffset: pixelOffset.x - pixelOffset.left, yOffset: pixelOffset.y - pixelOffset.top }
  placed.forEach(({ id, startBeat }, i) => {
    dispatch(addPlaylistItem({
      id, path: imported.files[i].path, startBeat, trackNumber: trackNumber + i, pixelOffset: offset,
    } as PlaylistItem))
  })
  const ids = placed.map(({ id }) => id)
  if(ids.length) dispatch(selectClips(ids))
  // dropped below the last track, or onto more than there are: new ones
  await refreshTrackCount()(dispatch)

  // each one's length, which the playlist needs to trim it
  await Promise.all(ids.map(async id => {
    try {
      const info = await invoke<ClipInfo>('get_clip', { id })
      dispatch(playlistSlice.actions.setClipInfo({ id, ...info }))
    } catch(e) {
      // eslint-disable-next-line no-console
      console.error(`could not load clip ${id}:`, e)
    }
  }))
  return ids
}
