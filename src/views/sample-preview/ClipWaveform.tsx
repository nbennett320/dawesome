import React from 'react'
import {
  ClipSettings,
  envelopeAt,
  gainFactor,
  playedDuration,
} from '../../types/clip'

export interface WaveformData {
  // [x, y] pairs from the engine (see calc_waveform_from_samples): x is the
  // point's index, y is scaled so the lowest sample is 0 and the highest is 1
  points: number[]
  // the sample's length in seconds, before any speed change
  duration: number
}

interface Props {
  data: WaveformData
  settings: ClipSettings
  // beats per minute, for the beat gridlines
  tempo: number
  // the attack or decay handle was dragged
  onEnvelopeChange: (change: Partial<Pick<ClipSettings, 'attackMs' | 'decayMs'>>) => void
}

const COLORS = {
  background: '#f8fafc',
  grid: 'rgba(0, 0, 0, 0.08)',
  beat: 'rgba(0, 0, 0, 0.35)',
  centre: 'rgba(0, 0, 0, 0.15)',
  wave: 'rgb(2, 132, 199)',
  fill: 'rgba(14, 165, 233, 0.15)',
  envelope: 'rgb(217, 119, 6)',
  fade: 'rgba(217, 119, 6, 0.08)',
}

// space for the beat labels along the top
const RULER_HEIGHT = 16
// how tall the wave is at 0 dB, as a fraction of the space either side of
// the centre line, leaving room to show gain boosts
const UNITY_HEIGHT = 0.6
// envelope handles: drawn radius, and how close counts as grabbing one
const HANDLE_RADIUS = 5
const HANDLE_HIT = 9

type Handle = 'attack' | 'decay'

// y values around their average, scaled so the loudest point reaches ±1
// (the engine's 0..1 scaling doesn't put silence at 0.5)
const centredLevels = (points: number[]): number[] => {
  const ys = points.filter((_, i) => i % 2 === 1)
  if(!ys.length) return []

  const mean = ys.reduce((a, b) => a + b, 0) / ys.length
  // a loop rather than Math.max(...ys): long samples have too many
  // points to pass as arguments
  const peak = ys.reduce((max, y) => Math.max(max, Math.abs(y - mean)), 0) || 1

  return ys.map(y => (y - mean) / peak)
}

interface Layout {
  width: number
  height: number
  // the clip's played length, seconds
  played: number
}

// where the envelope's top and bottom are drawn
const envelopeTop = () => RULER_HEIGHT + HANDLE_RADIUS + 2
const envelopeBottom = (height: number) => height - HANDLE_RADIUS - 2

// x of the attack handle (where the fade in ends) and the decay handle
// (where the fade out starts)
const handleX = (handle: Handle, settings: ClipSettings, { width, played }: Layout) => {
  if(played <= 0) return handle === 'attack' ? 0 : width
  return handle === 'attack'
    ? settings.attackMs / 1000 / played * width
    : width - settings.decayMs / 1000 / played * width
}

const draw = (
  ctx: CanvasRenderingContext2D,
  layout: Layout,
  levels: number[],
  settings: ClipSettings,
  tempo: number,
  active: Handle | null,
) => {
  const { width, height, played } = layout

  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = COLORS.background
  ctx.fillRect(0, 0, width, height)

  const top = RULER_HEIGHT
  const mid = top + (height - top) / 2
  const halfHeight = (height - top) / 2
  const attackX = handleX('attack', settings, layout)
  const decayX = handleX('decay', settings, layout)

  // shade the fades
  ctx.fillStyle = COLORS.fade
  ctx.fillRect(0, top, attackX, height - top)
  ctx.fillRect(decayX, top, width - decayX, height - top)

  // beat gridlines across the played length, labelled from 1 like the
  // playlist's timeline
  const beats = played * tempo / 60
  if(beats > 0) {
    const beatWidth = width / beats
    // skip labels that would overlap on a long clip
    const labelEvery = Math.max(1, Math.ceil(28 / beatWidth))

    ctx.font = '10px system-ui, sans-serif'
    ctx.textBaseline = 'middle'

    for(let b = 0; b <= beats; b++) {
      const x = Math.round(b * beatWidth) + 0.5

      ctx.strokeStyle = COLORS.grid
      ctx.beginPath()
      ctx.moveTo(x, top)
      ctx.lineTo(x, height)
      ctx.stroke()

      if(b % labelEvery === 0 && x < width - 8) {
        ctx.fillStyle = COLORS.beat
        ctx.fillText(String(b + 1), x + 3, RULER_HEIGHT / 2)
      }
    }
  }

  // centre line
  ctx.strokeStyle = COLORS.centre
  ctx.lineWidth = 1
  ctx.beginPath()
  ctx.moveTo(0, Math.round(mid) + 0.5)
  ctx.lineTo(width, Math.round(mid) + 0.5)
  ctx.stroke()

  if(levels.length >= 2) {
    const gain = gainFactor(settings.gainDb) * UNITY_HEIGHT
    const xAt = (i: number) => i / (levels.length - 1) * width
    const yAt = (i: number) => {
      const t = i / (levels.length - 1) * played
      const level = levels[i] * gain * envelopeAt(settings, t, played)
      // boosts past the top or bottom are drawn flat, like clipping
      return mid - Math.max(-1, Math.min(1, level)) * halfHeight
    }

    // shade between the wave and the centre line
    ctx.fillStyle = COLORS.fill
    ctx.beginPath()
    ctx.moveTo(0, mid)
    levels.forEach((_, i) => ctx.lineTo(xAt(i), yAt(i)))
    ctx.lineTo(width, mid)
    ctx.closePath()
    ctx.fill()

    ctx.strokeStyle = COLORS.wave
    ctx.beginPath()
    levels.forEach((_, i) => (i ? ctx.lineTo(xAt(i), yAt(i)) : ctx.moveTo(xAt(i), yAt(i))))
    ctx.stroke()
  }

  // the envelope: up from silence over the attack, down over the decay
  const eTop = envelopeTop()
  const eBottom = envelopeBottom(height)
  const envelopeY = (level: number) => eBottom - level * (eBottom - eTop)
  // where the fades cross when they overlap
  const peakX = Math.min(attackX, decayX)
  const peakLevel = played > 0 ? envelopeAt(settings, peakX / width * played, played) : 1

  ctx.strokeStyle = COLORS.envelope
  ctx.lineWidth = 1.5
  ctx.beginPath()
  ctx.moveTo(0, envelopeY(settings.attackMs > 0 ? 0 : 1))
  ctx.lineTo(peakX, envelopeY(peakLevel))
  if(decayX > attackX) ctx.lineTo(decayX, envelopeY(1))
  ctx.lineTo(width, envelopeY(settings.decayMs > 0 ? 0 : 1))
  ctx.stroke()

  // handles, at the top of the envelope
  const handles: Array<[Handle, number]> = [['attack', attackX], ['decay', decayX]]
  handles.forEach(([handle, x]) => {
    ctx.beginPath()
    ctx.arc(
      Math.max(HANDLE_RADIUS, Math.min(width - HANDLE_RADIUS, x)),
      eTop,
      HANDLE_RADIUS,
      0,
      Math.PI * 2,
    )
    ctx.fillStyle = handle === active ? COLORS.envelope : '#fff'
    ctx.fill()
    ctx.strokeStyle = COLORS.envelope
    ctx.stroke()
  })
}

// a clip's waveform as it will play: its gain, its fades (with handles to
// drag them), and its played length marked in beats
const ClipWaveform = ({ data, settings, tempo, onEnvelopeChange }: Props) => {
  const boxRef = React.useRef<HTMLDivElement>(null)
  const canvasRef = React.useRef<HTMLCanvasElement>(null)
  const [size, setSize] = React.useState({ width: 0, height: 0 })
  // the handle being dragged, or under the mouse
  const [dragging, setDragging] = React.useState<Handle | null>(null)
  const [hovered, setHovered] = React.useState<Handle | null>(null)

  const levels = React.useMemo(() => centredLevels(data.points), [data.points])
  const layout: Layout = {
    width: size.width,
    height: size.height,
    played: playedDuration(settings, data.duration),
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
  }, [size, levels, settings, data.duration, tempo, dragging, hovered])

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
    const { width, played } = layout
    const fraction = Math.max(0, Math.min(1, x / width))
    const seconds = handle === 'attack' ? fraction * played : (1 - fraction) * played
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
