// hearing a pitch on a track's instrument now (play_note, stop_note): a key
// of the Piano Roll's keyboard held down, or a short blip as a note is drawn
// or moved to another pitch
import { invoke } from '@tauri-apps/api/core'
import { NEW_VELOCITY } from './notes'

// how long a note drawn or moved sounds, unless the mouse is let go sooner
export const SHORT_MS = 200

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

export const playNote = (track: number, pitch: number, velocity = NEW_VELOCITY): Promise<void> =>
  invoke<void>('play_note', { track, pitch, velocity }).catch(logError('play the note'))

export const stopNote = (track: number, pitch: number): Promise<void> =>
  invoke<void>('stop_note', { track, pitch }).catch(logError('stop the note'))

// one pitch sounding at a time: a new one stops the last
export interface Auditioner {
  // start a pitch on a track's instrument, stopped by stop() or, if
  // `short`, after SHORT_MS
  play: (track: number, pitch: number, short: boolean) => void
  stop: () => void
  // the pitch sounding now, if any
  sounding: () => number | null
}

export const createAuditioner = (): Auditioner => {
  let held: { track: number, pitch: number, timer: ReturnType<typeof setTimeout> | null } | null = null

  const stop = () => {
    if(!held) return
    if(held.timer) clearTimeout(held.timer)
    stopNote(held.track, held.pitch)
    held = null
  }

  return {
    play: (track, pitch, short) => {
      stop()
      playNote(track, pitch)
      const timer = short ? setTimeout(stop, SHORT_MS) : null
      held = { track, pitch, timer }
    },
    stop,
    sounding: () => held?.pitch ?? null,
  }
}
