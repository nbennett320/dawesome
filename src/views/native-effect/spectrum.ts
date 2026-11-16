// an analyzer's picture over time. each poll brings the effect's spectrum as
// it is; the picture rises to it at once and falls away slowly, so peaks
// can be read. when nothing new has come (the effect isn't being run: its
// track has gone quiet and rests, or it's bypassed), the picture keeps
// falling, to the floor, rather than freezing
import { EffectSpectrum } from '../../types/nativeEffect'

export const FLOOR = -120
// how far the picture falls each poll, in dB
export const FALL = 1.5
// how often the engine is asked
export const POLL_MS = 33

export interface Picture {
  levels: number[]
  // the spectrum's `written` it was last fed
  written: number
}

// the picture after a poll. `got` is null when the engine said to ask again
// (the picture stays as it is)
export const follow = (picture: Picture | null, got: EffectSpectrum | null): Picture | null => {
  if(!got) return picture
  const fresh = !picture || got.written !== picture.written
  const levels = got.levels.map((level, i) => {
    const fallen = Math.max(FLOOR, (picture?.levels[i] ?? FLOOR) - FALL)
    return fresh ? Math.max(level, fallen) : fallen
  })
  return { levels, written: got.written }
}

// whether there's anything to draw (all at the floor: nothing)
export const showing = (picture: Picture | null): boolean =>
  !!picture && picture.levels.some(level => level > FLOOR)
