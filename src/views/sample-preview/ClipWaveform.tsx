import React from 'react'
import {
  ClipSettings,
  ClipWindow,
  envelopeAt,
  gainFactor,
  windowedDuration,
} from '../../types/clip'
import { painter, useThemeChange } from '../../theme/canvas'
import { Region, playedRegion, timeAtX } from './region'
import { clamp, clampLowWins } from '../../util/helpers'

export interface WaveformData {
  // the engine's levels at evenly spaced points (see
  // calc_waveform_from_samples), scaled so the lowest is 0 and the highest 1
  levels: Float32Array
  duration: number
}

interface Props {
  data: WaveformData
  settings: ClipSettings
  tempo: number
  // the part of the sample the clip plays, null for all of it
  window: ClipWindow | null
  // the attack or decay handle was dragged
  onEnvelopeChange: (change: Partial<Pick<ClipSettings, 'attackMs' | 'decayMs'>>) => void
}

// the colours come from the theme's waveform section
const paint = painter('waveform')

// space for the beat labels along the top
const RULER_HEIGHT = 16
// how tall the wave is at 0 dB, as a fraction of the space either side of
// the center line, leaving room to show gain boosts
const UNITY_HEIGHT = 0.6
// envelope handles: drawn radius, and how close counts as grabbing one
const HANDLE_RADIUS = 5
const HANDLE_HIT = 9

type Handle = 'attack' | 'decay'

// y values around their average, scaled so the loudest point reaches ±1
// (the engine's 0..1 scaling doesn't put silence at 0.5)
const centerdLevels = (ys: Float32Array): number[] => {
  if(!ys.length) return []

  const mean = ys.reduce((a, b) => a + b, 0) / ys.length
  // a loop rather than Math.max(...ys): long samples have too many
  // points to pass as arguments
  const peak = ys.reduce((max, y) => Math.max(max, Math.abs(y - mean)), 0) || 1

  return Array.from(ys, y => (y - mean) / peak)
}

interface Layout {
  width: number
  height: number
  // the clip's played length, seconds
  played: number
  // the part of the canvas the clip plays, the envelope sits inside it
  region: Region
}

// where the envelope's top and bottom are drawn
const envelopeTop = () => RULER_HEIGHT + HANDLE_RADIUS + 2
const envelopeBottom = (height: number) => height - HANDLE_RADIUS - 2

// x of the attack handle (where the fade in ends) and the decay handle
// (where the fade out starts)
const handleX = (handle: Handle, settings: ClipSettings, { played, region }: Layout) => {
  if(played <= 0) return handle === 'attack' ? region.left : region.left + region.width
  return handle === 'attack'
    ? region.left + settings.attackMs / 1000 / played * region.width
    : region.left + region.width - settings.decayMs / 1000 / played * region.width
}

const draw = (
  ctx: CanvasRenderingContext2D,
  layout: Layout,
  levels: number[],
  settings: ClipSettings,
  tempo: number,
  active: Handle | null,
) => {
  const { width, height, played, region } = layout
  const windowed = region.left > 0 || region.width < width

  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = paint.css('background')
  ctx.fillRect(0, 0, width, height)
  ctx.fillStyle = paint.css('ruler')
  ctx.fillRect(0, 0, width, RULER_HEIGHT)
  ctx.strokeStyle = paint.css('rulerLine')
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(0, RULER_HEIGHT - 0.5)
  ctx.lineTo(width, RULER_HEIGHT - 0.5)
  ctx.stroke()

  const top = RULER_HEIGHT
  const mid = top + (height - top) / 2
  const halfHeight = (height - top) / 2
  const attackX = handleX('attack', settings, layout)
  const decayX = handleX('decay', settings, layout)

  // shade the fades
  ctx.fillStyle = paint.css('fade')
  ctx.fillRect(region.left, top, attackX - region.left, height - top)
  ctx.fillRect(decayX, top, region.left + region.width - decayX, height - top)

  // beat gridlines across the played length, labelled from 1 like the
  // playlist's timeline. beat 1 is the region's left edge, and the lines
  // go on across the rest of the canvas at the same spacing
  const beats = played * tempo / 60
  if(beats > 0 && region.width > 0) {
    const beatWidth = region.width / beats
    // skip labels that would overlap on a long clip
    const labelEvery = Math.max(1, Math.ceil(28 / beatWidth))

    ctx.font = '10px system-ui, sans-serif'
    ctx.textBaseline = 'middle'

    const firstBeat = Math.ceil(-region.left / region.width * beats)
    const lastBeat = Math.floor((width - region.left) / region.width * beats)

    for(let b = firstBeat; b <= lastBeat; b++) {
      const x = Math.round(region.left + b * beatWidth) + 0.5

      ctx.strokeStyle = paint.css('grid')
      ctx.beginPath()
      ctx.moveTo(x, top)
      ctx.lineTo(x, height)
      ctx.stroke()

      if(b >= 0 && b % labelEvery === 0 && x < width - 8) {
        ctx.fillStyle = paint.css('beat')
        ctx.fillText(String(b + 1), x + 3, RULER_HEIGHT / 2)
      }
    }
  }

  // center line
  ctx.strokeStyle = paint.css('center')
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(0, Math.round(mid) + 0.5)
  ctx.lineTo(width, Math.round(mid) + 0.5)
  ctx.stroke()

  if(levels.length >= 2) {
    const gain = gainFactor(settings.gainDb) * UNITY_HEIGHT
    const xAt = (i: number) => i / (levels.length - 1) * width
    // boosts past the top or bottom are drawn flat, like clipping
    const heightAt = (level: number) => mid - clamp(level, -1, 1) * halfHeight
    const yAt = (i: number) => {
      const t = timeAtX(region, played, xAt(i))
      return heightAt(levels[i] * gain * envelopeAt(settings, t, played))
    }
    const strokeWave = (y: (i: number) => number) => {
      ctx.beginPath()
      levels.forEach((_, i) => (i ? ctx.lineTo(xAt(i), y(i)) : ctx.moveTo(xAt(i), y(i))))
      ctx.stroke()
    }

    if(windowed) {
      // outside the region: unshaped by the envelope, muted, no fill
      ctx.save()
      ctx.beginPath()
      ctx.rect(0, top, region.left, height - top)
      ctx.rect(region.left + region.width, top, width - region.left - region.width, height - top)
      ctx.clip()
      ctx.strokeStyle = paint.css('dimmed')
      strokeWave(i => heightAt(levels[i] * gain))
      ctx.restore()

      // inside the region only
      ctx.save()
      ctx.beginPath()
      ctx.rect(region.left, top, region.width, height - top)
      ctx.clip()
    }

    // shade between the wave and the center line
    ctx.fillStyle = paint.css('fill')
    ctx.beginPath()
    ctx.moveTo(0, mid)
    levels.forEach((_, i) => ctx.lineTo(xAt(i), yAt(i)))
    ctx.lineTo(width, mid)
    ctx.closePath()
    ctx.fill()

    ctx.strokeStyle = paint.css('wave')
    strokeWave(yAt)

    if(windowed) ctx.restore()
  }

  if(windowed) {
    // dim what the clip doesn't play, and mark where it starts and ends
    ctx.fillStyle = paint.css('wash')
    ctx.fillRect(0, top, region.left, height - top)
    ctx.fillRect(region.left + region.width, top, width - region.left - region.width, height - top)

    ctx.strokeStyle = paint.css('rulerLine')
    ctx.lineWidth = 1
    const edges = [region.left, region.left + region.width]
    edges.filter(x => x > 0 && x < width).forEach(x => {
      ctx.beginPath()
      ctx.moveTo(Math.round(x) + 0.5, top)
      ctx.lineTo(Math.round(x) + 0.5, height)
      ctx.stroke()
    })
  }

  // the envelope: up from silence over the attack, down over the decay
  const eTop = envelopeTop()
  const eBottom = envelopeBottom(height)
  const envelopeY = (level: number) => eBottom - level * (eBottom - eTop)
  // where the fades cross when they overlap
  const peakX = Math.min(attackX, decayX)
  const peakLevel = played > 0 ? envelopeAt(settings, timeAtX(region, played, peakX), played) : 1

  ctx.strokeStyle = paint.css('envelope')
  ctx.lineWidth = 1.5
  ctx.beginPath()
  ctx.moveTo(region.left, envelopeY(settings.attackMs > 0 ? 0 : 1))
  ctx.lineTo(peakX, envelopeY(peakLevel))
  if(decayX > attackX) ctx.lineTo(decayX, envelopeY(1))
  ctx.lineTo(region.left + region.width, envelopeY(settings.decayMs > 0 ? 0 : 1))
  ctx.stroke()

  // handles, at the top of the envelope
  const handles: Array<[Handle, number]> = [['attack', attackX], ['decay', decayX]]
  handles.forEach(([handle, x]) => {
    ctx.beginPath()
    ctx.arc(
      // kept inside the played part (in the middle of one too narrow)
      region.width < HANDLE_RADIUS * 2
        ? region.left + region.width / 2
        : clampLowWins(x, region.left + HANDLE_RADIUS, region.left + region.width - HANDLE_RADIUS),
      eTop,
      HANDLE_RADIUS,
      0,
      Math.PI * 2,
    )
    ctx.fillStyle = handle === active ? paint.css('envelope') : paint.css('background')
    ctx.fill()
    ctx.strokeStyle = paint.css('envelope')
    ctx.stroke()
  })
}

// a clip's waveform as it will play: its gain, its fades (with handles to
// drag them), and its played length marked in beats
const ClipWaveform = ({ data, settings, tempo, window: clipWindow, onEnvelopeChange }: Props) => {
  const boxRef = React.useRef<HTMLDivElement>(null)
  const canvasRef = React.useRef<HTMLCanvasElement>(null)
  const themeChange = useThemeChange()
  const [size, setSize] = React.useState({ width: 0, height: 0 })
  // the handle being dragged, or under the mouse
  const [dragging, setDragging] = React.useState<Handle | null>(null)
  const [hovered, setHovered] = React.useState<Handle | null>(null)

  const levels = React.useMemo(() => centerdLevels(data.levels), [data.levels])
  const layout: Layout = {
    width: size.width,
    height: size.height,
    played: windowedDuration(settings, data.duration, clipWindow),
    region: playedRegion(clipWindow, data.duration, size.width),
  }

  // follow the container's size: window resizes, split pane dividers
  React.useEffect(() => {
    const box = boxRef.current
    if(!box) return undefined

    const observer = new ResizeObserver(() => {
      const { width, height } = box.getBoundingClientRect()
      // hidden behind another tab, keep the last size
      if(width && height) setSize({ width, height })
    })
    observer.observe(box)

    return () => observer.disconnect()
  }, [])

  React.useEffect(() => {
    const canvas = canvasRef.current
    const ctx = canvas?.getContext('2d')
    if(!canvas || !ctx || !size.width || !size.height) return

    // draw at the screen's pixel density so lines stay sharp
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.round(size.width * dpr)
    canvas.height = Math.round(size.height * dpr)
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

    draw(ctx, layout, levels, settings, tempo, dragging ?? hovered)
  }, [size, levels, settings, data.duration, clipWindow, tempo, dragging, hovered, themeChange])

  // pointer position on the canvas
  const pointerX = (ev: { clientX: number }) =>
    ev.clientX - (canvasRef.current?.getBoundingClientRect().left ?? 0)
  const pointerY = (ev: { clientY: number }) =>
    ev.clientY - (canvasRef.current?.getBoundingClientRect().top ?? 0)

  // the handle under the pointer, the nearest if both are close
  const handleAt = (x: number, y: number): Handle | null => {
    if(Math.abs(y - envelopeTop()) > HANDLE_HIT) return null

    const near = (['attack', 'decay'] as Handle[])
      .map(h => [h, Math.abs(x - handleX(h, settings, layout))] as const)
      .filter(([, dist]) => dist <= HANDLE_HIT)
      .sort((a, b) => a[1] - b[1])

    return near.length ? near[0][0] : null
  }

  // milliseconds for a handle dragged to x
  const msAt = (handle: Handle, x: number) => {
    const { played, region } = layout
    // pointer held inside the region
    const time = timeAtX(region, played, x)
    const seconds = handle === 'attack' ? time : played - time
    return Math.round(seconds * 1000)
  }

  return (
    <div ref={boxRef} className='relative w-full h-full overflow-hidden'>
      <canvas
        ref={canvasRef}
        className='absolute inset-0'
        style={{
          width: size.width,
          height: size.height,
          cursor: dragging || hovered ? 'ew-resize' : 'default',
          touchAction: 'none',
        }}
        onPointerDown={ev => {
          const handle = handleAt(pointerX(ev), pointerY(ev))
          if(!handle) return

          ev.currentTarget.setPointerCapture(ev.pointerId)
          setDragging(handle)
        }}
        onPointerMove={ev => {
          const x = pointerX(ev)

          if(dragging) {
            onEnvelopeChange(dragging === 'attack'
              ? { attackMs: msAt('attack', x) }
              : { decayMs: msAt('decay', x) })
          } else {
            setHovered(handleAt(x, pointerY(ev)))
          }
        }}
        onPointerUp={() => setDragging(null)}
        onPointerCancel={() => setDragging(null)}
        onPointerLeave={() => setHovered(null)}
        onDoubleClick={ev => {
          // double clicking a handle removes that fade
          const handle = handleAt(pointerX(ev), pointerY(ev))
          if(handle) onEnvelopeChange(handle === 'attack' ? { attackMs: 0 } : { decayMs: 0 })
        }}
      />
    </div>
  )
}

export default ClipWaveform
