import { Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import { syncPlayingFrom } from './slices/playlistSlice'
import { sendLatest } from './slices/mixerSlice'

// scrubbing (part S in context/tape-project): dragging the playhead (the
// timeline, the playlist's ruler) or the Transport tab's reels plays the
// song at the drag's speed, either way, and holding Rewind or Fast forward
// plays it as it winds. normal playback pauses meanwhile, and carries on
// from where it's let go if it was playing (see engine/src/daw/scrub)

const scrubError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// which drag it is: Settings → Controls → Tape → Scrubbing playback says
// which are heard (the engine leaves the rest silent: a seek when let go)
export type ScrubSource = 'timeline' | 'playlist' | 'reels'

// a drag begins at `atMs` on the timeline
export const scrubBegin = (atMs: number, source: ScrubSource) =>
  invoke<void>('scrub_begin', { atMs, source }).catch(scrubError('scrub'))

// the drag's at `atMs` now (sent at most every few ms, the latest last)
export const scrubTo = (atMs: number) =>
  sendLatest('scrub', () => invoke<void>('scrub_to', { atMs }).catch(scrubError('scrub')))

// let go at `beat`: the playhead moves there, and playing carries on from
// there if it was playing. settles with the beat it moved to
export const scrubEnd = (beat: number) => (dispatch: Dispatch) =>
  invoke<number>('scrub_end', { beat })
    .then(moved => syncPlayingFrom(dispatch).then(() => moved))
    .catch(e => {
      scrubError('let go of the scrub')(e)
      return beat
    })

// a wind key held: the song heard winding from `atMs` at `speed` (multiples
// of play speed, negative back)
export const windBegin = (atMs: number, speed: number) => (dispatch: Dispatch) =>
  invoke<void>('wind_begin', { atMs, speed })
    .then(() => syncPlayingFrom(dispatch))
    .catch(scrubError('wind'))

// let go of the wind key: the playhead's left where the tape got to (the
// engine knows; `fallbackMs` if it doesn't). settles with where, in ms
export const windEnd = (fallbackMs: number) => (dispatch: Dispatch) =>
  invoke<number>('wind_end', { fallbackMs })
    .then(ms => syncPlayingFrom(dispatch).then(() => ms))
    .catch(e => {
      scrubError('stop winding')(e)
      return fallbackMs
    })
