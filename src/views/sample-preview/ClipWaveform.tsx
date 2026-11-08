import React from 'react'

export interface WaveformData {
  // [x, y] pairs from the engine (see calc_waveform_from_samples): x is the
  // point's index, y is scaled so the lowest sample is 0 and the highest is 1
  points: number[]
  // seconds
  duration: number
}

interface Props {
  data: WaveformData
  // beats per minute, for the beat gridlines
  tempo: number
}

const COLORS = {
  background: '#f8fafc',
  grid: 'rgba(0, 0, 0, 0.08)',
  beat: 'rgba(0, 0, 0, 0.35)',
  centre: 'rgba(0, 0, 0, 0.15)',
  wave: 'rgb(2, 132, 199)',
  fill: 'rgba(14, 165, 233, 0.15)',
}

// space for the beat labels along the top
const RULER_HEIGHT = 16

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

const draw = (
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  levels: number[],
  duration: number,
  tempo: number,
) => {
  ctx.clearRect(0, 0, width, height)
  ctx.fillStyle = COLORS.background
  ctx.fillRect(0, 0, width, height)

  const top = RULER_HEIGHT
  const mid = top + (height - top) / 2
  // leave a little room above and below the loudest point
  const amp = (height - top) / 2 * 0.9

  // beat gridlines, labelled from 1 like the playlist's timeline
  const beats = duration * tempo / 60
  if(beats > 0) {
    const beatWidth = width / beats
    // skip labels that would overlap when zoomed out on a long clip
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
  ctx.beginPath()
  ctx.moveTo(0, Math.round(mid) + 0.5)
  ctx.lineTo(width, Math.round(mid) + 0.5)
  ctx.stroke()

  if(levels.length < 2) return

  const xAt = (i: number) => i / (levels.length - 1) * width
  const yAt = (i: number) => mid - levels[i] * amp

  // shade between the wave and the centre line
  ctx.fillStyle = COLORS.fill
  ctx.beginPath()
  ctx.moveTo(0, mid)
  levels.forEach((_, i) => ctx.lineTo(xAt(i), yAt(i)))
  ctx.lineTo(width, mid)
  ctx.closePath()
  ctx.fill()

  ctx.strokeStyle = COLORS.wave
  ctx.lineWidth = 1
  ctx.beginPath()
  levels.forEach((_, i) => (i ? ctx.lineTo(xAt(i), yAt(i)) : ctx.moveTo(xAt(i), yAt(i))))
  ctx.stroke()
}

// a clip's waveform, filling its container, with the clip's beats marked
const ClipWaveform = ({ data, tempo }: Props) => {
  const boxRef = React.useRef<HTMLDivElement>(null)
  const canvasRef = React.useRef<HTMLCanvasElement>(null)
  const [size, setSize] = React.useState({ width: 0, height: 0 })

  const levels = React.useMemo(() => centredLevels(data.points), [data.points])

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

    draw(ctx, size.width, size.height, levels, data.duration, tempo)
  }, [size, levels, data.duration, tempo])

  return (
    <div ref={boxRef} className='relative w-full h-full overflow-hidden'>
      <canvas
        ref={canvasRef}
        className='absolute inset-0'
        style={{ width: size.width, height: size.height }}
      />
    </div>
  )
}

export default ClipWaveform
