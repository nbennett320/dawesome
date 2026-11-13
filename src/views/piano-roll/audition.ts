// hearing a pitch on a row now, straight to the master (play_row_note,
// stop_row_note): a key of the Piano Roll's keyboard held down, or a short
// blip as a note is drawn or moved to another pitch
import { invoke } from '@tauri-apps/api/core'
import { NEW_VELOCITY } from './notes'

// how long a note drawn or moved sounds, unless the mouse is let go sooner
export const SHORT_MS = 200

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

export const playNote = (rowId: number, pitch: number, velocity = NEW_VELOCITY): Promise<void> =>
  invoke<void>('play_row_note', { rowId, pitch, velocity }).catch(logError('play the note'))

export const stopNote = (rowId: number, pitch: number): Promise<void> =>
  invoke<void>('stop_row_note', { rowId, pitch }).catch(logError('stop the note'))

// one pitch sounding at a time: a new one stops the last
export interface Auditioner {
  // start a pitch on a row, stopped by stop() or, if `short`, after
  // SHORT_MS
  play: (rowId: number, pitch: number, short: boolean) => void
  stop: () => void
  // the pitch sounding now, if any
  sounding: () => number | null
}

export const createAuditioner = (): Auditioner => {
  let held: { rowId: number, pitch: number, timer: ReturnType<typeof setTimeout> | null } | null = null

  const stop = () => {
    if(!held) return
    if(held.timer) clearTimeout(held.timer)
    stopNote(held.rowId, held.pitch)
    held = null
  }

  return {
    play: (rowId, pitch, short) => {
      stop()
      playNote(rowId, pitch)
      const timer = short ? setTimeout(stop, SHORT_MS) : null
      held = { rowId, pitch, timer }
    },
    stop,
    sounding: () => held?.pitch ?? null,
  }
}
