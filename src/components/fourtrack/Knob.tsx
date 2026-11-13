import React from 'react'
import { useControllable } from './util'
import { clamp, cx } from '../../util/helpers'

export interface KnobProps {
  value?: number
  defaultValue?: number
  onChange?: (value: number) => void
  min?: number
  max?: number
  step?: number
  // what double-clicking or the "0" key goes back to. the midpoint by default
  center?: number
  // the whole sweep in degrees
  sweep?: number
  // pixels of vertical drag from min to max
  dragRange?: number
  size?: number
  label: string
  // the value for screen readers, e.g. "L 40"
  formatValue?: (value: number) => string
  disabled?: boolean
  className?: string
  title?: string
  // drawn in the middle of the cap, turning with it, saying what the knob does
  icon?: React.ReactNode
}

// drag up and down (shift for fine moves), arrow keys (shift for big steps),
// Home/End, or double-click to centre
export const Knob = ({
  value,
  defaultValue,
  onChange,
  min = -1,
  max = 1,
  step = 0.01,
  center = (min + max) / 2,
  sweep = 270,
  dragRange = 180,
  size = 64,
  label,
  formatValue,
  disabled,
  className,
  title,
  icon,
}: KnobProps) => {
  const [v, setV] = useControllable(value, defaultValue ?? center, onChange)
  const drag = React.useRef<{ y: number, v: number } | null>(null)

  const commit = (next: number) => {
    const snapped = Math.round(clamp(next, min, max) / step) * step
    setV(Number(snapped.toFixed(6)))
  }

  const t = (v - min) / (max - min)
  const angle = (t - 0.5) * sweep
  const coarse = (max - min) / 10
  const fine = step * 5

  const endDrag = () => { drag.current = null }

  return (
    <div
      role='slider'
      tabIndex={disabled ? -1 : 0}
      aria-label={label}
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={v}
      aria-valuetext={formatValue?.(v)}
      aria-disabled={disabled || undefined}
      title={title}
      className={cx('ft-knob', icon ? 'ft-knob--icon' : undefined, className)}
      style={{ width: size, height: size, opacity: disabled ? 0.5 : undefined }}
      onPointerDown={e => {
        if(disabled || e.button !== 0) return
        e.currentTarget.setPointerCapture(e.pointerId)
        e.currentTarget.focus()
        drag.current = { y: e.clientY, v }
      }}
      onPointerMove={e => {
        if(!drag.current) return
        const scale = e.shiftKey ? 0.25 : 1
        const next = drag.current.v + ((drag.current.y - e.clientY) / dragRange) * (max - min) * scale
        commit(next)
        // fine moves are relative to here, so switching shift doesn't jump
        if(e.shiftKey) drag.current = { y: e.clientY, v: next }
      }}
      onPointerUp={endDrag}
      onPointerCancel={endDrag}
      onDoubleClick={() => { if(!disabled) commit(center) }}
      onKeyDown={e => {
        if(disabled) return
        const s = e.shiftKey ? coarse : fine
        const next = ({
          ArrowUp: v + s,
          ArrowRight: v + s,
          ArrowDown: v - s,
          ArrowLeft: v - s,
          PageUp: v + coarse,
          PageDown: v - coarse,
          Home: min,
          End: max,
          0: center,
        } as Record<string, number | undefined>)[e.key]
        if(next !== undefined) {
          e.preventDefault()
          commit(next)
        }
      }}
    >
      <span className='ft-knob__rot' style={{ transform: `rotate(${angle}deg)` }}>
        <span className='ft-knob__cap' />
        <span className='ft-knob__ptr' />
        {icon && <span className='ft-knob__icon'>{icon}</span>}
      </span>
    </div>
  )
}
