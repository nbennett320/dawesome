// the knob boxes under a modulation effect's view, and the drag on the
// view itself
import React from 'react'
import { AnalogButton } from '../../../components/fourtrack'
import { choiceNormalized, ParamWire } from '../../../types/nativeEffect'
import ParamControl from '../ParamControl'
import type { EffectParamsApi } from '../useEffectParams'

// (`name` shows it under another name: the Filter's Decay is Release in
// Dynamic mode)
export const Control = ({ param, api, name }: { param: ParamWire | undefined, api: EffectParamsApi, name?: string }) => {
  if(!param) return null
  const shown = name ? { ...param, name } : param
  return (
    <ParamControl
      param={shown}
      label={shown.name === shown.group ? shown.name : `${shown.group} ${shown.name}`}
      set={api.set}
      setText={api.setText}
    />
  )
}

// a choice as a row of keys, the one that's on held down (as the Delay's
// Mode)
export const ChoiceKeys = ({ param, api, dim = false }: { param: ParamWire | undefined, api: EffectParamsApi, dim?: boolean }) => {
  if(!param) return null
  return (
    <div className={`flex flex-col items-center gap-1 ${dim ? 'opacity-40' : ''}`}>
      <span className='ft-label'>{param.name}</span>
      <div role='group' aria-label={param.name} className='flex flex-row gap-0.5'>
        {param.labels.map((label, index) => (
          <AnalogButton
            key={label}
            variant='text'
            size='xs'
            latched={Math.round(param.value.value) === index}
            aria-label={`${param.name} ${label}`}
            onPress={() => api.set(param.id, choiceNormalized(param, index))}
          >
            {label.toUpperCase()}
          </AnalogButton>
        ))}
      </div>
    </div>
  )
}

export const Group = ({ name, children }: { name: string, children: React.ReactNode }) => (
  <section aria-label={name} className='ft-strip flex flex-col gap-2 px-2 pt-1.5 pb-2'>
    <h3 className='ft-label'>{name}</h3>
    <div className='flex flex-row flex-wrap items-end gap-1'>{children}</div>
  </section>
)

// the Experimental box: dashed, so it reads as extra, with a key that
// puts all of its parameters back to their defaults (off) at once
export const ExperimentalGroup = ({ params, api }: { params: Array<ParamWire | undefined>, api: EffectParamsApi }) => {
  const present = params.filter((p): p is ParamWire => !!p)
  const on = present.some(p => Math.abs(p.value.normalized - p.defaultNormalized) > 1e-6)
  return (
    <section
      aria-label='Experimental'
      className='flex flex-col gap-2 px-2 pt-1.5 pb-2 rounded border border-dashed border-line-strong'
    >
      <div className='flex flex-row items-center justify-between gap-2'>
        <h3 className='ft-label'>Experimental</h3>
        <AnalogButton
          variant='text'
          size='xs'
          aria-label='Turn every experimental parameter off'
          disabled={!on}
          onPress={() => present.forEach(p => api.set(p.id, p.defaultNormalized))}
        >
          OFF
        </AnalogButton>
      </div>
      <div className='flex flex-row flex-wrap items-end gap-1'>
        {present.map(p => <Control key={p.id} param={p} api={api} />)}
      </div>
    </section>
  )
}

// shift-drags move this much of the way
const FINE = 0.25

interface Drag {
  x: number
  y: number
  across: number
  up: number
  fine: boolean
}

// dragging on a view: across moves `across` (its whole range over the
// view's width), up moves `up` (over its height). shift: fine. one undo
// step, as the mouse press is
export const useXYDrag = (
  api: EffectParamsApi,
  across: ParamWire | undefined,
  up: ParamWire | undefined,
  size: { width: number, height: number },
) => {
  const drag = React.useRef<Drag | null>(null)
  const [dragging, setDragging] = React.useState(false)

  const take = (x: number, y: number, fine: boolean): Drag => ({
    x, y, fine, across: across?.value.normalized ?? 0, up: up?.value.normalized ?? 0,
  })
  const at = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect()
    return { x: e.clientX - rect.left, y: e.clientY - rect.top }
  }
  const clamp = (n: number) => Math.min(1, Math.max(0, n))

  const onPointerDown = (e: React.PointerEvent<HTMLCanvasElement>) => {
    if(e.button !== 0) return
    const { x, y } = at(e)
    e.currentTarget.setPointerCapture?.(e.pointerId)
    drag.current = take(x, y, e.shiftKey)
    setDragging(true)
  }

  const onPointerMove = (e: React.PointerEvent<HTMLCanvasElement>) => {
    const d = drag.current
    if(!d) return
    const { x, y } = at(e)
    if(e.shiftKey !== d.fine) {
      drag.current = take(x, y, e.shiftKey)
      return
    }
    const scale = d.fine ? FINE : 1
    if(across) {
      const n = clamp(d.across + ((x - d.x) / Math.max(1, size.width)) * scale)
      if(Math.abs(n - across.value.normalized) > 1e-6) api.set(across.id, n)
    }
    if(up) {
      const n = clamp(d.up - ((y - d.y) / Math.max(1, size.height)) * scale)
      if(Math.abs(n - up.value.normalized) > 1e-6) api.set(up.id, n)
    }
  }

  const end = () => {
    drag.current = null
    setDragging(false)
  }

  return {
    dragging,
    handlers: { onPointerDown, onPointerMove, onPointerUp: end, onPointerCancel: end },
  }
}
