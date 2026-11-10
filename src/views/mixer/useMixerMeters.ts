import React from 'react'
import { invoke } from '@tauri-apps/api'
import { dbToPosition, gainToDb, MIN_DB, MixerLevels } from '../../types/mixer'

// the meters' parts the hook draws into (see Meter.tsx)
export interface MeterElements {
  bar: HTMLDivElement
  hold: HTMLDivElement
}

export type RegisterMeter = (key: string, elements: MeterElements | null) => void

// the key a track's meter is registered under
export const trackMeterKey = (trackNumber: number) => `track-${trackNumber}`
export const MASTER_METER_KEY = 'master'

// how often the engine's levels are read while playing
const POLL_MS = 33
// how fast a meter falls back once the sound gets quieter
const FALL_DB_PER_SECOND = 24
// how long the peak line stays put before it falls
const HOLD_MS = 1000

interface Reading {
  db: number
  holdDb: number
  holdUntil: number
}

// reads the engine's levels while playing and draws them into the meters
//
// this skips Redux and React rendering: the meters are redrawn every frame,
// straight into their elements, so the rest of the app isn't re-rendered
// 30 times a second
export const useMixerMeters = (playing: boolean) => {
  const meters = React.useRef(new Map<string, MeterElements>())
  const readings = React.useRef(new Map<string, Reading>())
  // whether the master went over 0 dBFS since the light was cleared
  const [clipped, setClipped] = React.useState(false)

  const register = React.useCallback<RegisterMeter>((key, elements) => {
    if(elements) meters.current.set(key, elements)
    else meters.current.delete(key)
  }, [])

  // a new reading jumps the meter up. it falls back on its own
  const feed = (key: string, peak: number, now: number) => {
    const db = gainToDb(peak)
    const reading = readings.current.get(key) ?? { db: MIN_DB, holdDb: MIN_DB, holdUntil: 0 }

    if(db >= reading.db) reading.db = db
    if(db >= reading.holdDb) {
      reading.holdDb = db
      reading.holdUntil = now + HOLD_MS
    }
    readings.current.set(key, reading)
  }

  React.useEffect(() => {
    if(!playing) return undefined

    let active = true
    // one reading at a time, if the engine is slow to answer
    let waiting = false

    const poll = async () => {
      if(waiting) return
      waiting = true

      try {
        const levels = await invoke<MixerLevels>('get_mixer_levels', {})
        if(!active) return

        const now = performance.now()
        levels.tracks.forEach(([trackNumber, peak]) => feed(trackMeterKey(trackNumber), peak, now))
        feed(MASTER_METER_KEY, levels.master, now)
        if(levels.master > 1) setClipped(true)
      } catch(e) {
        // eslint-disable-next-line no-console
        console.error('could not read the mixer levels:', e)
      } finally {
        waiting = false
      }
    }

    const timer = setInterval(poll, POLL_MS)
    return () => {
      active = false
      clearInterval(timer)
    }
  }, [playing])

  // the meters are drawn every frame while there's something to show: while
  // playing, and until they've fallen back to silence after. then the loop
  // stops, until playback starts again
  React.useEffect(() => {
    let frame = 0
    let last = performance.now()

    const draw = (now: number) => {
      const fall = FALL_DB_PER_SECOND * Math.max(0, now - last) / 1000
      last = now
      let moving = false

      meters.current.forEach((elements, key) => {
        const reading = readings.current.get(key)
        let level = 0
        let hold = 0

        if(reading) {
          reading.db = Math.max(MIN_DB, reading.db - fall)
          if(now > reading.holdUntil) reading.holdDb = Math.max(MIN_DB, reading.holdDb - fall)
          level = dbToPosition(reading.db)
          hold = dbToPosition(reading.holdDb)
          if(reading.db > MIN_DB || reading.holdDb > MIN_DB) moving = true
        }

        // the bar's colours span the whole meter, so it's uncovered rather
        // than stretched
        elements.bar.style.clipPath = `inset(${(1 - level) * 100}% 0 0 0)`
        elements.hold.style.bottom = `${hold * 100}%`
        elements.hold.style.opacity = hold > 0 ? '1' : '0'
      })

      // (this frame drew them at rest, so they stay that way)
      frame = playing || moving ? requestAnimationFrame(draw) : 0
    }

    frame = requestAnimationFrame(draw)
    return () => cancelAnimationFrame(frame)
  }, [playing])

  const clearClip = React.useCallback(() => setClipped(false), [])

  return { register, clipped, clearClip }
}
