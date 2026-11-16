// the Delay's own panel: its echo view, and below it the Time, Echoes, Tone
// and Mix controls. the view draws each echo as a bar, worked out from the
// settings (echoes.ts), so it follows a knob at once
//
// on the view: drag the first echo left or right for the time (to the
// nearest note value when synced), and up or down for the feedback. shift:
// fine. a drag is one undo step
import React from 'react'
import { AnalogButton } from '../../../components/fourtrack'
import { useAppSelector } from '../../../hooks/redux'
import { selectBarQuarters, selectPlaylistTempo } from '../../../state/slices/playlistSlice'
import { painter, useThemeChange } from '../../../theme/canvas'
import { choiceNormalized, normalizedOf, ParamWire } from '../../../types/nativeEffect'
import ParamControl from '../ParamControl'
import type { EffectParamsApi } from '../useEffectParams'
import { drawDelay, Scene } from './draw'
import {
  captionOf, echoesOf, FEEDBACK, HIGH_CUT, levels, longerSide, LOW_CUT, MIX, MODE, nearestNote, NOTE, OFFSET,
  offsetFactor, Settings, settingsOf, spanOf, SYNC, TIME,
} from './echoes'
import { Area, HIT, timeToX, xToTime } from './geometry'

// shift-drags move this much of the way
const FINE = 0.25
// the shortest time a drag sets, in seconds
const SHORTEST = 0.001

interface Drag {
  // the side of the echo taken
  side: 0 | 1
  // the span shown when it was taken: kept while dragging, so the view
  // doesn't rescale under the pointer
  span: number
  // where the pointer was, and the echo's time and the feedback then;
  // taken again when shift is pressed or let go, so switching doesn't jump
  x: number
  y: number
  time: number
  feedback: number
  fine: boolean
}

// Stereo | Ping-pong: a key each, the one that's on held down
const ModeKeys = ({ param, set }: { param: ParamWire, set: EffectParamsApi['set'] }) => (
  <div className='flex flex-col items-center gap-1'>
    <span className='ft-label'>{param.name}</span>
    <div role='group' aria-label={param.name} className='flex flex-row gap-0.5'>
      {param.labels.map((label, index) => (
        <AnalogButton
          key={label}
          variant='text'
          size='xs'
          latched={Math.round(param.value.value) === index}
          aria-label={label}
          onPress={() => set(param.id, choiceNormalized(param, index))}
        >
          {label.toUpperCase()}
        </AnalogButton>
      ))}
    </div>
  </div>
)

// the time of the first echo on `side`, in seconds
const firstTime = (scene: Scene, side: 0 | 1): number | null =>
  scene.echoes.find(e => e.first && e.side === side)?.time ?? null

const DelayPanel = ({ api }: { api: EffectParamsApi, slotId: number, active: boolean }) => {
  useThemeChange()
  const tempo = useAppSelector(selectPlaylistTempo)
  const barQuarters = useAppSelector(selectBarQuarters)
  const { params, set, setText } = api
  const settings = settingsOf(params)

  const [area, setArea] = React.useState<Area>({ width: 0, height: 0 })
  const [hovering, setHovering] = React.useState(false)
  const [dragging, setDragging] = React.useState(false)
  const box = React.useRef<HTMLDivElement>(null)
  const canvas = React.useRef<HTMLCanvasElement>(null)
  const drag = React.useRef<Drag | null>(null)

  // the canvas fills its box
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

  const sceneOf = (s: Settings): Scene => {
    const span = drag.current?.span ?? spanOf(s, tempo)
    const { dry } = levels(s.mix)
    return {
      area,
      span,
      echoes: echoesOf(s, tempo, span),
      dryDb: dry > 0 ? 20 * Math.log10(dry) : -Infinity,
      synced: s.sync,
      beat: 60 / (tempo > 0 ? tempo : 120),
      barQuarters,
      caption: captionOf(s, tempo),
      hot: hovering || dragging,
    }
  }
  const scene = settings ? sceneOf(settings) : null

  React.useEffect(() => {
    const el = canvas.current
    const ctx = el?.getContext('2d')
    if(!el || !ctx || !scene || area.width === 0 || area.height === 0) return
    const scale = window.devicePixelRatio || 1
    const [width, height] = [Math.round(area.width * scale), Math.round(area.height * scale)]
    if(el.width !== width || el.height !== height) {
      el.width = width
      el.height = height
    }
    ctx.setTransform(scale, 0, 0, scale, 0, 0)
    drawDelay(ctx, scene)
  })

  if(!settings || !scene) return null

  const param = (id: number): ParamWire => params[id]

  // where a pointer event is, in the area's pixels
  const at = (e: { clientX: number, clientY: number }) => {
    const rect = canvas.current!.getBoundingClientRect()
    return { x: e.clientX - rect.left, y: e.clientY - rect.top }
  }

  // the side of the first echo near `x`, if one is
  const hitFirst = (x: number): 0 | 1 | null => {
    let best: { side: 0 | 1, off: number } | null = null
    scene.echoes.filter(e => e.first).forEach(e => {
      const off = Math.abs(timeToX(e.time, scene.span, area) - x)
      if(off <= HIT && (!best || off < best.off)) best = { side: e.side, off }
    })
    return (best as { side: 0 | 1 } | null)?.side ?? null
  }

  const take = (side: 0 | 1, x: number, y: number, fine: boolean, span: number): Drag | null => {
    const time = firstTime(scene, side)
    if(time === null) return null
    return { side, span, x, y, time, feedback: param(FEEDBACK).value.normalized, fine }
  }

  const onPointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if(e.button !== 0) return
    const { x, y } = at(e)
    const side = hitFirst(x)
    if(side === null) return
    const taken = take(side, x, y, e.shiftKey, scene.span)
    if(!taken) return
    e.currentTarget.setPointerCapture?.(e.pointerId)
    drag.current = taken
    setDragging(true)
  }

  // the base time (before offset) that puts the dragged echo at `time`
  const setTime = (d: Drag, time: number) => {
    const stretched = d.side === longerSide(settings) ? offsetFactor(settings) : 1
    const base = Math.max(SHORTEST, time / stretched)
    if(settings.sync) {
      const note = param(NOTE)
      const index = nearestNote(note.labels, base, tempo)
      if(index !== Math.round(note.value.value)) set(NOTE, choiceNormalized(note, index))
    } else {
      // (only a change: moving straight up or down leaves the time be)
      const time = normalizedOf(param(TIME), base * 1000)
      if(Math.abs(time - param(TIME).value.normalized) > 1e-6) set(TIME, time)
    }
  }

  const onPointerMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const { x, y } = at(e)
    const d = drag.current
    if(!d) {
      setHovering(hitFirst(x) !== null)
      return
    }
    // shift pressed or let go: carry on from here
    if(e.shiftKey !== d.fine) {
      drag.current = take(d.side, x, y, e.shiftKey, d.span) ?? d
      return
    }
    const scale = d.fine ? FINE : 1
    setTime(d, d.time + (xToTime(x, d.span, area) - xToTime(d.x, d.span, area)) * scale)
    const feedback = Math.min(1, Math.max(0, d.feedback - ((y - d.y) / Math.max(1, area.height / 2)) * scale))
    if(feedback !== param(FEEDBACK).value.normalized) set(FEEDBACK, feedback)
  }

  const endDrag = () => {
    drag.current = null
    setDragging(false)
  }

  const control = (id: number) => {
    const p = param(id)
    return (
      <ParamControl
        key={p.id}
        param={p}
        // ("Time", not "Time Time")
        label={p.name === p.group ? p.name : `${p.group} ${p.name}`}
        set={set}
        setText={setText}
      />
    )
  }

  const group = (name: string, children: React.ReactNode) => (
    <section aria-label={name} className='ft-strip flex flex-col gap-2 px-2 pt-1.5 pb-2'>
      <h3 className='ft-label'>{name}</h3>
      <div className='flex flex-row flex-wrap items-end gap-1'>{children}</div>
    </section>
  )

  return (
    <div className='h-full flex flex-col min-h-0'>
      <div ref={box} className='relative flex-1 min-h-[140px] m-2 mb-0 rounded overflow-hidden border border-line'>
        <canvas
          ref={canvas}
          aria-label='Delay echoes. Drag the first echo: left and right for the time, up and down for the feedback'
          className='absolute inset-0 w-full h-full outline-none'
          style={{
            cursor: dragging ? 'grabbing' : hovering ? 'grab' : 'default',
            backgroundColor: painter('effect').css('background'),
          }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onPointerLeave={() => { if(!drag.current) setHovering(false) }}
        />
      </div>

      <div className='shrink-0 flex flex-row flex-wrap gap-2 p-2'>
        {group('Time', [control(SYNC), control(settings.sync ? NOTE : TIME), control(OFFSET)])}
        {group('Echoes', [<ModeKeys key='mode' param={param(MODE)} set={set} />, control(FEEDBACK)])}
        {group('Tone', [control(LOW_CUT), control(HIGH_CUT)])}
        {group('Mix', [control(MIX)])}
      </div>
    </div>
  )
}

export default DelayPanel
