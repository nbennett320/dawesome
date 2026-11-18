// drawing the Chorus's view, "ghost strings": the dry sound as a string
// across each side's lane (left above, right below), and each voice a
// glowing copy of it, slid along by how late it reads now. the copies sway
// against the string and each other as the LFO turns: the shimmer. a voice
// panned one way is brighter in that side's lane. Tone softens the copies,
// Bloom fades them in after an attack, Prism tints each with its slice
import { painter } from '../../../theme/canvas'
import { RGBA } from '../../../theme/color'
import { blend, css, spectrumAt } from '../modulation/motion'
import type { Area } from '../modulation/useLiveCanvas'
import { panGains, Settings, sliceOf, SWING, Voice } from './voices'

export interface Scene {
  area: Area
  settings: Settings
  voices: Voice[]
  // 0 to 1, the LFO's phase (for the dial)
  phase: number
  // how open Bloom has the copies (1 without Bloom)
  open: number
  caption: string
  hot: boolean
}

// cycles of the string across the view
const CYCLES = 3.5
const PAD = 18

// how bright the strings are, 0 (Tone at 1 kHz) to 1 (at 20 kHz)
export const brightness = (toneHz: number): number =>
  Math.min(1, Math.max(0, Math.log(toneHz / 1000) / Math.log(20)))

// a string's height at `t` (0 to 1 across), a cycle every 1/CYCLES: a
// sine with some of its third harmonic, more of it when brighter
export const stringAt = (t: number, bright: number): number => {
  const x = 2 * Math.PI * CYCLES * t
  return (Math.sin(x) + 0.35 * bright * Math.sin(3 * x + 0.4)) / (1 + 0.35 * bright)
}

// how far a voice's copy is slid along (share of the width): a little for
// its delay, and a quarter cycle at the most for its swing, so even a
// shallow chorus visibly sways
export const slideOf = (delay: number, voice: Voice, s: Settings): number => {
  const swing = SWING * s.delayMs
  const swayed = swing > 0 ? (delay - voice.center) / swing : 0
  return (0.12 * voice.center / 40 + 0.25 * swayed) / CYCLES
}

const drawString = (
  ctx: CanvasRenderingContext2D, area: Area, mid: number, amp: number, slide: number, bright: number,
) => {
  ctx.beginPath()
  const steps = Math.max(32, Math.round(area.width / 3))
  for(let i = 0; i <= steps; i += 1) {
    const t = i / steps
    const x = PAD + t * (area.width - 2 * PAD)
    const y = mid - amp * stringAt(t - slide, bright)
    if(i === 0) ctx.moveTo(x, y)
    else ctx.lineTo(x, y)
  }
  ctx.stroke()
}

export const drawChorus = (ctx: CanvasRenderingContext2D, scene: Scene) => {
  const { area, settings: s, voices, open } = scene
  const effect = painter('effect')
  ctx.clearRect(0, 0, area.width, area.height)
  ctx.fillStyle = effect.css('background')
  ctx.fillRect(0, 0, area.width, area.height)

  const lane = (area.height - PAD) / 2
  const mids = [PAD + lane / 2, PAD + lane * 1.5]
  const amp = Math.max(4, lane * 0.32)
  const bright = brightness(s.toneHz)
  const accent: RGBA = effect.rgba('curve')

  // the lanes' middles and names
  ctx.strokeStyle = effect.css('grid')
  ctx.lineWidth = 1
  ctx.beginPath()
  mids.forEach(m => { ctx.moveTo(0, Math.round(m) + 0.5); ctx.lineTo(area.width, Math.round(m) + 0.5) })
  ctx.stroke()
  ctx.font = '10px system-ui, sans-serif'
  ctx.fillStyle = effect.css('label')
  ctx.fillText('L', 5, mids[0] - lane / 2 + 12)
  ctx.fillText('R', 5, mids[1] - lane / 2 + 12)

  // the dry string
  ctx.strokeStyle = effect.css('zero')
  ctx.lineWidth = 1.25
  mids.forEach(m => drawString(ctx, area, m, amp, 0, bright))

  // the copies: glowing, softer as Tone falls
  const wet = Math.min(1, 2 * s.mix)
  ctx.lineWidth = 1.5
  ctx.shadowBlur = 2 + 8 * bright
  voices.forEach(voice => {
    const gains = panGains(voice.pan)
    ;[0, 1].forEach(side => {
      const level = voice.gain * (gains[side] / Math.SQRT2) * open * (0.3 + 0.7 * wet)
      if(level <= 0.01) return
      const color = blend(accent, spectrumAt(0.85 * sliceOf(voice.index, voices.length, side)), s.prism)
      ctx.strokeStyle = css(color, 0.25 + 0.6 * level)
      ctx.shadowColor = css(color, 0.6 * level)
      // (side by side a few pixels apart, so the voices don't sit as one)
      const lift = (voice.index - (voices.length - 1) / 2) * 4
      drawString(ctx, area, mids[side] + lift, amp, slideOf(voice.delays[side], voice, s), bright)
    })
  })
  ctx.shadowBlur = 0

  // the LFO's dial, bottom right: a dot going round
  const r = 9
  const [cx, cy] = [area.width - r - 8, area.height - r - 8]
  ctx.strokeStyle = effect.css('grid')
  ctx.beginPath()
  ctx.arc(cx, cy, r, 0, 2 * Math.PI)
  ctx.stroke()
  const angle = 2 * Math.PI * scene.phase - Math.PI / 2
  ctx.fillStyle = effect.css('curve')
  ctx.beginPath()
  ctx.arc(cx + r * Math.cos(angle), cy + r * Math.sin(angle), 2.5, 0, 2 * Math.PI)
  ctx.fill()

  ctx.fillStyle = effect.css(scene.hot ? 'curve' : 'label')
  ctx.fillText(scene.caption, PAD, 12)
}
