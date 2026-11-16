// the EQ's own panel: its curve, which bands are edited on, and below it the
// band chips, the selected band's controls and the output
//
// on the curve:
// - double-click empty space: a new band there (a cut at either end, else a
//   bell at the gain clicked)
// - drag a dot: its frequency, and its gain (shapes with one). shift: fine
// - scroll on a dot: its Q
// - right-click a dot: its shape, or Remove
// - Delete or Backspace: remove the selected band
// a band that's removed is switched off: its slot is free for the next
import React from 'react'
import { createPortal } from 'react-dom'
import { painter, useThemeChange } from '../../../theme/canvas'
import { newGesture } from '../../../state/gesture'
import { isDeleteKey } from '../../../util/keys'
import { choiceNormalized, normalizedOf, ParamWire } from '../../../types/nativeEffect'
import ClipMenu from '../../playlist/ClipMenu'
import ParamControl from '../ParamControl'
import type { EffectParamsApi } from '../useEffectParams'
import { useEffectSpectrum } from '../useEffectSpectrum'
import { Picture, showing } from '../spectrum'
import { BAND_COLORS, drawEq, Scene } from './draw'
import {
  Area, freeBand, hitBand, newBandShape, xToFreq, xToFreqNormalized, yToDb, yToGainNormalized,
} from './geometry'
import {
  bandsOf, FREQ, GAIN, hasGain, ON, OUTPUT, paramId, Q, SHAPE, Shape,
} from './response'

// how much a scroll's pixel turns Q (normalized)
const WHEEL_Q = 0.0015
// scrolls this close together are one undo step
const WHEEL_GESTURE_MS = 400
// shift-drags move this much of the way
const FINE = 0.25

const SHAPE_NAMES = ['Bell', 'Low shelf', 'High shelf', 'Low cut', 'High cut', 'Notch', 'Band-pass']

interface Drag {
  band: number
  // where the pointer was, and the band's values then (normalized); taken
  // again when shift is pressed or let go, so switching doesn't jump
  x: number
  y: number
  freq: number
  gain: number
  fine: boolean
}

interface Menu {
  band: number
  clientX: number
  clientY: number
}

// the band chips: one per band, in its color, lit when it's on. picking one
// selects it (an off one too, to switch it on from its controls)
const BandChips = ({ on, selected, onSelect }: {
  on: boolean[]
  selected: number | null
  onSelect: (band: number) => void
}) => {
  const track = painter('track')
  return (
    <div className='flex flex-row flex-wrap gap-1' role='group' aria-label='Bands'>
      {on.map((lit, band) => (
        <button
          // eslint-disable-next-line react/no-array-index-key
          key={band}
          type='button'
          aria-pressed={band === selected}
          aria-label={`Band ${band + 1}${lit ? '' : ' (off)'}`}
          title={lit ? `Band ${band + 1}` : `Band ${band + 1}: off. Select it to switch it on`}
          className='w-6 h-6 rounded-full text-[10px] font-bold text-surface border-2 transition-opacity'
          style={{
            backgroundColor: track.css(BAND_COLORS[band]),
            opacity: lit ? 1 : 0.35,
            borderColor: band === selected ? painter('effect').css('selectedRing') : 'transparent',
          }}
          onClick={() => onSelect(band)}
        >
          {band + 1}
        </button>
      ))}
    </div>
  )
}

const EqPanel = ({ api, slotId, active }: { api: EffectParamsApi, slotId: number, active: boolean }) => {
  useThemeChange()
  const { params, set } = api
  const bands = bandsOf(params)
  const output = params[OUTPUT] as ParamWire | undefined

  const [selected, setSelected] = React.useState<number | null>(() => bands.find(b => b.on)?.index ?? null)
  const [menu, setMenu] = React.useState<Menu | null>(null)
  const [area, setArea] = React.useState<Area>({ width: 0, height: 0 })
  const [hovering, setHovering] = React.useState(false)
  const [dragging, setDragging] = React.useState(false)
  const box = React.useRef<HTMLDivElement>(null)
  const canvas = React.useRef<HTMLCanvasElement>(null)
  const drag = React.useRef<Drag | null>(null)
  const wheel = React.useRef<{ gesture: number, at: number } | null>(null)

  const param = (band: number, which: number): ParamWire => params[paramId(band, which)]

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

  const caption = selected === null || !bands[selected]?.on ? null : [
    param(selected, SHAPE).value.text,
    param(selected, FREQ).value.text,
    ...(hasGain(bands[selected].shape) ? [param(selected, GAIN).value.text] : []),
    `Q ${param(selected, Q).value.text}`,
  ].join(' · ')

  // what's drawn: the panel's own (as of this render), and the spectrum,
  // which comes many times a second and is drawn without React
  const scene = React.useRef<Omit<Scene, 'spectrum'> | null>(null)
  const picture = React.useRef<Picture | null>(null)
  scene.current = { area, bands, outputDb: output?.value.value ?? 0, selected, caption }
  const paint = React.useCallback(() => {
    const el = canvas.current
    const ctx = el?.getContext('2d')
    const now = scene.current
    if(!el || !ctx || !now || now.area.width === 0 || now.area.height === 0) return
    const scale = window.devicePixelRatio || 1
    const [width, height] = [Math.round(now.area.width * scale), Math.round(now.area.height * scale)]
    if(el.width !== width || el.height !== height) {
      el.width = width
      el.height = height
    }
    ctx.setTransform(scale, 0, 0, scale, 0, 0)
    const levels = showing(picture.current) ? picture.current!.levels : null
    drawEq(ctx, { ...now, spectrum: levels })
  }, [])
  useEffectSpectrum(slotId, active, got => {
    picture.current = got
    paint()
  })
  React.useEffect(paint)

  // where a pointer event is, in the area's pixels
  const at = (e: { clientX: number, clientY: number }) => {
    const rect = canvas.current!.getBoundingClientRect()
    return { x: e.clientX - rect.left, y: e.clientY - rect.top }
  }

  const addBand = (x: number, y: number) => {
    const band = freeBand(bands)
    if(band === null) return
    const freq = xToFreq(x, area)
    const shape = newBandShape(freq)
    const gain = shape === Shape.Bell ? yToDb(y, area) : 0
    // one undo step for the lot
    const gesture = newGesture()
    set(paramId(band, SHAPE), choiceNormalized(param(band, SHAPE), shape), gesture)
    set(paramId(band, FREQ), xToFreqNormalized(x, area), gesture)
    set(paramId(band, GAIN), normalizedOf(param(band, GAIN), gain), gesture)
    set(paramId(band, Q), param(band, Q).defaultNormalized, gesture)
    set(paramId(band, ON), 1, gesture)
    setSelected(band)
  }

  const removeBand = (band: number) => {
    set(paramId(band, ON), 0, newGesture())
    if(band === selected) setSelected(bands.find(b => b.on && b.index !== band)?.index ?? null)
  }

  const setShape = (band: number, shape: Shape) => {
    set(paramId(band, SHAPE), choiceNormalized(param(band, SHAPE), shape), newGesture())
  }

  const onPointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if(e.button !== 0) return
    const { x, y } = at(e)
    const band = hitBand(bands, x, y, area, selected)
    if(band === null) return
    setSelected(band)
    e.currentTarget.setPointerCapture?.(e.pointerId)
    setDragging(true)
    drag.current = {
      band,
      x,
      y,
      freq: param(band, FREQ).value.normalized,
      gain: param(band, GAIN).value.normalized,
      fine: e.shiftKey,
    }
  }

  const onPointerMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const { x, y } = at(e)
    const d = drag.current
    if(!d) {
      setHovering(hitBand(bands, x, y, area, selected) !== null)
      return
    }
    // shift pressed or let go: carry on from here
    if(e.shiftKey !== d.fine) {
      drag.current = {
        ...d, x, y, fine: e.shiftKey, freq: param(d.band, FREQ).value.normalized, gain: param(d.band, GAIN).value.normalized,
      }
      return
    }
    const scale = d.fine ? FINE : 1
    const clamp = (n: number) => Math.min(1, Math.max(0, n))
    const freq = clamp(d.freq + (xToFreqNormalized(x, area) - xToFreqNormalized(d.x, area)) * scale)
    set(paramId(d.band, FREQ), freq)
    if(hasGain(bands[d.band].shape)) {
      const gain = clamp(d.gain + (yToGainNormalized(y, area) - yToGainNormalized(d.y, area)) * scale)
      set(paramId(d.band, GAIN), gain)
    }
  }

  const endDrag = () => {
    drag.current = null
    setDragging(false)
  }

  const onDoubleClick = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = at(e)
    if(hitBand(bands, x, y, area, selected) !== null) return
    addBand(x, y)
  }

  const onContextMenu = (e: React.MouseEvent<HTMLCanvasElement>) => {
    const { x, y } = at(e)
    const band = hitBand(bands, x, y, area, selected)
    if(band === null) return
    e.preventDefault()
    setSelected(band)
    setMenu({ band, clientX: e.clientX, clientY: e.clientY })
  }

  // (a native listener, below: React's can't stop the pane scrolling)
  const onWheel = (e: WheelEvent) => {
    const { x, y } = at(e)
    const band = hitBand(bands, x, y, area, selected)
    if(band === null) return
    // the scroll is the dot's, not the pane's
    e.preventDefault()
    // a burst of scrolling is one undo step
    const now = Date.now()
    if(!wheel.current || now - wheel.current.at > WHEEL_GESTURE_MS) wheel.current = { gesture: newGesture(), at: now }
    wheel.current.at = now
    const q = param(band, Q).value.normalized
    // scrolling up sharpens it
    set(paramId(band, Q), Math.min(1, Math.max(0, q - e.deltaY * WHEEL_Q)), wheel.current.gesture)
  }

  const wheelHandler = React.useRef(onWheel)
  wheelHandler.current = onWheel
  React.useEffect(() => {
    const el = canvas.current
    if(!el) return undefined
    const listener = (e: WheelEvent) => wheelHandler.current(e)
    el.addEventListener('wheel', listener, { passive: false })
    return () => el.removeEventListener('wheel', listener)
  }, [])

  const onKeyDown = (e: React.KeyboardEvent) => {
    if(isDeleteKey(e) && !e.repeat && selected !== null && bands[selected].on) {
      e.preventDefault()
      removeBand(selected)
    }
  }

  const label = (band: number, p: ParamWire) => `Band ${band + 1} ${p.name}`

  return (
    <div className='h-full flex flex-col min-h-0'>
      <div ref={box} className='relative flex-1 min-h-[140px] m-2 mb-0 rounded overflow-hidden border border-line'>
        <canvas
          ref={canvas}
          aria-label='EQ curve. Double-click to add a band, drag a band to move it, scroll on it for Q, right-click it for its shape'
          tabIndex={0}
          className='absolute inset-0 w-full h-full outline-none'
          style={{ cursor: dragging ? 'grabbing' : hovering ? 'grab' : 'crosshair' }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onDoubleClick={onDoubleClick}
          onContextMenu={onContextMenu}
          onKeyDown={onKeyDown}
        />
      </div>

      <div className='shrink-0 flex flex-row flex-wrap gap-2 p-2'>
        <section aria-label='Bands' className='ft-strip flex flex-col gap-2 px-2 pt-1.5 pb-2'>
          <h3 className='ft-label'>Bands</h3>
          <BandChips on={bands.map(b => b.on)} selected={selected} onSelect={setSelected} />
        </section>

        {selected !== null && (
          <section aria-label={`Band ${selected + 1}`} className='ft-strip flex flex-col gap-2 px-2 pt-1.5 pb-2'>
            <h3 className='ft-label'>Band {selected + 1}</h3>
            <div className='flex flex-row flex-wrap gap-1'>
              {[ON, SHAPE, FREQ, GAIN, Q].map(which => {
                const p = param(selected, which)
                return <ParamControl key={p.id} param={p} label={label(selected, p)} set={set} setText={api.setText} />
              })}
            </div>
          </section>
        )}

        {output && (
          <section aria-label='Output' className='ft-strip flex flex-col gap-2 px-2 pt-1.5 pb-2'>
            <h3 className='ft-label'>Output</h3>
            <ParamControl param={output} label='Output' set={set} setText={api.setText} />
          </section>
        )}
      </div>

      {menu && createPortal(
        <div className='fixed inset-0 z-50 pointer-events-none [&>*]:pointer-events-auto'>
          <ClipMenu
            x={menu.clientX}
            y={menu.clientY}
            items={[
              ...SHAPE_NAMES.map((name, shape) => ({
                label: `${bands[menu.band].shape === shape ? '✓ ' : ''}${name}`,
                onSelect: () => setShape(menu.band, shape as Shape),
              })),
              'divider' as const,
              { label: 'Remove', danger: true, shortcut: '⌫', onSelect: () => removeBand(menu.band) },
            ]}
            onClose={() => setMenu(null)}
          />
        </div>,
        document.body,
      )}
    </div>
  )
}

export default EqPanel

