import React from 'react'
import { useVisible } from '../useEffectSpectrum'
import { Reading, useEffectReadout } from '../useEffectReadout'

export interface Area {
  width: number
  height: number
}

// a canvas that fills its box and is drawn every display frame while
// `active` (its tab showing) and the window can be seen, with the effect's
// latest readout (useEffectReadout). `draw` is called with the 2D context
// (scaled to the area's own pixels), the area, the latest reading (null
// until one comes) and the time (performance.now()). drawn once more
// whenever `deps` change, so a knob shows while it isn't animating
export const useLiveCanvas = (
  slotId: number,
  active: boolean,
  draw: (ctx: CanvasRenderingContext2D, area: Area, reading: Reading | null, now: number) => void,
  deps: unknown[],
) => {
  const box = React.useRef<HTMLDivElement>(null)
  const canvas = React.useRef<HTMLCanvasElement>(null)
  const [area, setArea] = React.useState<Area>({ width: 0, height: 0 })
  const reading = React.useRef<Reading | null>(null)
  const painter = React.useRef(draw)
  painter.current = draw
  const visible = useVisible()

  useEffectReadout(slotId, active, got => { reading.current = got })

  React.useEffect(() => {
    const el = box.current
    if(!el) return undefined
    const measure = () => setArea({ width: el.clientWidth, height: el.clientHeight })
    measure()
    if(typeof ResizeObserver === 'undefined') return undefined
    const observer = new ResizeObserver(measure)
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  const paint = React.useCallback((now: number) => {
    const el = canvas.current
    const ctx = el?.getContext('2d')
    if(!el || !ctx || area.width === 0 || area.height === 0) return
    const scale = window.devicePixelRatio || 1
    const [width, height] = [Math.round(area.width * scale), Math.round(area.height * scale)]
    if(el.width !== width || el.height !== height) {
      el.width = width
      el.height = height
    }
    ctx.setTransform(scale, 0, 0, scale, 0, 0)
    painter.current(ctx, area, reading.current, now)
  }, [area])

  // (each change of settings)
  // eslint-disable-next-line react-hooks/exhaustive-deps
  React.useEffect(() => { paint(performance.now()) }, [paint, ...deps])

  React.useEffect(() => {
    if(!active || !visible || typeof requestAnimationFrame === 'undefined') return undefined
    let frame = requestAnimationFrame(function tick(now) {
      paint(now)
      frame = requestAnimationFrame(tick)
    })
    return () => cancelAnimationFrame(frame)
  }, [active, visible, paint])

  return { box, canvas, area }
}
