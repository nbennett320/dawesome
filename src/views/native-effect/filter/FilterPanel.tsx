// the Filter's own panel: its curve (draw.ts), live as it moves, over its
// output's spectrum; below it the Filter, Motion, Output and Experimental
// controls, Motion showing only the mode that's on
//
// on the view: drag the Cutoff handle (or anywhere) across for the cutoff
// and up or down for the resonance; drag the Floor handle across for the
// floor. shift: fine. a drag is one undo step
import React from 'react'
import { painter, useThemeChange } from '../../../theme/canvas'
import { ParamWire } from '../../../types/nativeEffect'
import { ChoiceKeys, Control, ExperimentalGroup, Group } from '../modulation/ModControls'
import { liveAt, paramOf } from '../modulation/motion'
import { useLiveCanvas } from '../modulation/useLiveCanvas'
import { Picture, showing } from '../spectrum'
import type { EffectParamsApi } from '../useEffectParams'
import { useEffectSpectrum } from '../useEffectSpectrum'
import { captionOf, freqsAt, MODE, settingsOf, SLOT } from './curve'
import { Curve, curveOf, drawFilter, freqToX, HIT, PAD, TRAIL } from './draw'

const FINE = 0.25
// how long an attack's flash lasts, ms
const FLASH_MS = 150

type Handle = 'floor' | 'cutoff'

interface Drag {
  handle: Handle
  x: number
  y: number
  across: number
  up: number
  fine: boolean
}

const FilterPanel = ({ api, slotId, active }: { api: EffectParamsApi, slotId: number, active: boolean }) => {
  const theme = useThemeChange()
  const { params, set } = api
  const settings = settingsOf(params)
  const p = (key: string) => paramOf(params, key)
  const noteLabel = p('note')?.labels[Math.round(p('note')?.value.value ?? 0)] ?? ''
  const [hot, setHot] = React.useState<Handle | null>(null)
  const drag = React.useRef<Drag | null>(null)
  const trail = React.useRef<Curve[]>([])
  const spectrum = React.useRef<Picture | null>(null)
  const flash = React.useRef({ triggers: -1, at: -Infinity })

  useEffectSpectrum(slotId, active, got => { spectrum.current = got })

  const { box, canvas, area } = useLiveCanvas(slotId, active, (ctx, size, reading, now) => {
    if(!settings) return
    const live = liveAt(reading, now)
    const [l, r] = freqsAt(settings, live)
    const left = curveOf(l, settings, size)
    const triggers = live?.slots[SLOT.TRIGGERS] ?? 0
    if(triggers !== flash.current.triggers) {
      // (not for the count it had when the panel opened)
      if(flash.current.triggers >= 0) flash.current.at = now
      flash.current.triggers = triggers
    }
    drawFilter(ctx, {
      area: size,
      settings,
      left,
      right: settings.spin > 0 ? curveOf(r, settings, size) : null,
      trail: trail.current,
      freq: l,
      spectrum: showing(spectrum.current) ? spectrum.current!.levels : null,
      flash: settings.mode === MODE.ENVELOPE ? Math.exp(-(now - flash.current.at) / FLASH_MS) : 0,
      level: live?.slots[SLOT.M_L] ?? 0,
      phase: live?.phase ?? 0,
      caption: captionOf(settings, noteLabel),
      hot,
    })
    trail.current = live?.running && settings.mode !== MODE.OFF ? [...trail.current, left].slice(-TRAIL) : []
  }, [params, theme, hot])

  if(!settings) return null

  // the handle near `x`, if one is (Floor only while something moves it)
  const handleAt = (x: number): Handle | null => {
    const near = (hz: number) => Math.abs(freqToX(hz, area) - x) <= HIT
    if(settings.mode !== MODE.OFF && near(settings.floor)) return 'floor'
    if(near(settings.cutoff)) return 'cutoff'
    return null
  }
  const at = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    return { x: e.clientX - rect.left, y: e.clientY - rect.top }
  }
  const take = (handle: Handle, x: number, y: number, fine: boolean): Drag => ({
    handle, x, y, fine,
    across: p(handle)?.value.normalized ?? 0,
    up: p('resonance')?.value.normalized ?? 0,
  })
  const move = (param: ParamWire | undefined, to: number) => {
    if(!param) return
    const n = Math.min(1, Math.max(0, to))
    if(Math.abs(n - param.value.normalized) > 1e-6) set(param.id, n)
  }

  const onPointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if(e.button !== 0) return
    const { x, y } = at(e)
    e.currentTarget.setPointerCapture?.(e.pointerId)
    drag.current = take(handleAt(x) ?? 'cutoff', x, y, e.shiftKey)
    setHot(drag.current.handle)
  }
  const onPointerMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const { x, y } = at(e)
    const d = drag.current
    if(!d) {
      const near = handleAt(x)
      if(near !== hot) setHot(near)
      return
    }
    if(e.shiftKey !== d.fine) {
      drag.current = take(d.handle, x, y, e.shiftKey)
      return
    }
    const scale = d.fine ? FINE : 1
    // (the axis is the frequency's own range, by octave: a pixel across is
    // the same share of it)
    move(p(d.handle), d.across + ((x - d.x) / Math.max(1, area.width - 2 * PAD)) * scale)
    if(d.handle === 'cutoff') move(p('resonance'), d.up - ((y - d.y) / Math.max(1, area.height)) * scale)
  }
  const end = () => {
    drag.current = null
    setHot(null)
  }

  const motion = () => {
    switch(settings.mode) {
      case MODE.LFO: return [
        <Control key='sync' param={p('sync')} api={api} />,
        <Control key='rate' param={settings.sync ? p('note') : p('rate')} api={api} />,
        <Control key='shape' param={p('shape')} api={api} />,
      ]
      case MODE.ENVELOPE: return [
        <Control key='attack' param={p('attack')} api={api} />,
        <Control key='decay' param={p('decay')} api={api} />,
        <Control key='sensitivity' param={p('sensitivity')} api={api} />,
      ]
      case MODE.DYNAMIC: return [
        <Control key='attack' param={p('attack')} api={api} />,
        <Control key='decay' param={p('decay')} api={api} name='Release' />,
        <Control key='sensitivity' param={p('sensitivity')} api={api} />,
      ]
      default: return []
    }
  }

  return (
    <div className='h-full flex flex-col min-h-0'>
      <div ref={box} className='relative flex-1 min-h-[140px] m-2 mb-0 rounded overflow-hidden border border-line'>
        <canvas
          ref={canvas}
          aria-label='Filter curve. Drag across for the cutoff, up and down for the resonance; drag the F handle for the floor'
          className='absolute inset-0 w-full h-full outline-none'
          style={{ cursor: drag.current ? 'grabbing' : hot ? 'grab' : 'ew-resize', backgroundColor: painter('effect').css('background') }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={end}
          onPointerCancel={end}
          onPointerLeave={() => { if(!drag.current) setHot(null) }}
        />
      </div>

      <div className='shrink-0 flex flex-row flex-wrap gap-2 p-2'>
        <Group name='Filter'>
          <ChoiceKeys param={p('type')} api={api} />
          <ChoiceKeys param={p('slope')} api={api} />
          <Control param={p('cutoff')} api={api} />
          <Control param={p('resonance')} api={api} />
          <Control param={p('drive')} api={api} />
        </Group>
        <Group name='Motion'>
          <ChoiceKeys param={p('mode')} api={api} />
          {settings.mode !== MODE.OFF && <Control param={p('floor')} api={api} />}
          {motion()}
        </Group>
        <Group name='Output'>
          <Control param={p('mix')} api={api} />
          <Control param={p('output')} api={api} />
        </Group>
        <ExperimentalGroup params={[p('vowel'), p('spin'), p('tune')]} api={api} />
      </div>
    </div>
  )
}

export default FilterPanel
