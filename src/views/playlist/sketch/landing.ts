// a jump to a landmark (⌘←/⌘→, Home, End): the playhead glides there
// rather than jumping, and the ruler flashes where it lands. single steps
// stay instant. with experimental animations on (and the glide not with
// reduced motion). plain functions of the time since the jump, in ms

import { clamp } from '../../../util/helpers'

export const LANDING = {
  // how long the playhead takes to get there
  glideMs: 120,
  // how long the flash takes to fade
  flashMs: 600,
} as const

// how far along the glide is, 0 to 1, `elapsed` ms after the jump. eased
// out, so it leaves quickly and settles
export const glideProgress = (elapsed: number): number => {
  const t = clamp(elapsed / LANDING.glideMs, 0, 1)
  return 1 - (1 - t) ** 3
}

// where the playhead is drawn, gliding from `from` to `to`
export const glideBeat = (from: number, to: number, elapsed: number): number =>
  from + (to - from) * glideProgress(elapsed)

// how strong the flash is, 1 to 0, `elapsed` ms after the jump: bright at
// once, then fading, quickly at first
export const flashStrength = (elapsed: number): number => {
  const t = clamp(elapsed / LANDING.flashMs, 0, 1)
  return (1 - t) ** 2
}
