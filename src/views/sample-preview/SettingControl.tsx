import React from 'react'
import { Knob } from '../../components/fourtrack'
import { clamp } from '../../util/helpers'
import { ReadoutInput } from '../../components/common'

interface Props {
  label: string
  value: number
  min: number
  max: number
  step: number
  // shown after the number field, e.g. 'dB'
  unit?: string
  // the number field shows the value multiplied by this (pan is -1..1,
  // shown as -100..100 %)
  fieldScale?: number
  // a short description of the value, shown under the field
  hint?: string
  // > 1 gives the knob more room at the low end, for ranges like fade
  // lengths where small values matter most. the value is
  // min + (max - min) * position ^ taper
  taper?: number
  onChange: (value: number) => void
  // double clicking the label or knob calls this
  onReset: () => void
}

// round away floating point noise for display
const tidy = (n: number) => Number(n.toFixed(4))


// one setting as a strip: its label, a knob, and a number field to type into.
// double-click the label or knob to reset it
const SettingControl = ({
  label,
  value,
  min,
  max,
  step,
  unit,
  fieldScale = 1,
  hint,
  taper = 1,
  onChange,
  onReset,
}: Props) => {
  // the knob turns through 0..1, mapped onto the range through the taper
  const range = max - min || 1
  const toPosition = (v: number) => (clamp(v, min, max) - min) / range
  const position = taper === 1 ? toPosition(value) : toPosition(value) ** (1 / taper)
  const fromPosition = (p: number) => {
    const v = min + range * (taper === 1 ? p : p ** taper)
    return clamp(Math.round(v / step) * step, min, max)
  }

  const text = `${tidy(value * fieldScale)}${unit ? ` ${unit}` : ''}`

  return (
    <div className='ft-strip flex flex-col items-center gap-2 px-3 pt-2.5 pb-2 w-32'>
      <span
        className='ft-label'
        title='Double-click to reset'
        onDoubleClick={onReset}
      >
        {label}
      </span>

      {/* double-clicking resets to the setting's default, rather than the
          knob's centre */}
      <div
        onDoubleClickCapture={e => {
          e.stopPropagation()
          onReset()
        }}
      >
        <Knob
          label={label}
          title='Drag up or down (shift for fine moves), or double-click to reset'
          size={44}
          min={0}
          max={1}
          step={0.001}
          value={position}
          onChange={p => onChange(fromPosition(p))}
          formatValue={() => text}
        />
      </div>

      <div className='w-full flex flex-row items-center gap-1'>
        <ReadoutInput
          className='flex-1 min-w-0'
          value={String(tidy(value * fieldScale))}
          aria-label={`${label} value`}
          onCommit={text => {
            const typed = parseFloat(text)
            if(Number.isFinite(typed)) onChange(clamp(typed / fieldScale, min, max))
          }}
        />
        {unit && <span className='ft-label normal-case tracking-normal w-5'>{unit}</span>}
      </div>

      <span className='h-4 text-[10px] text-ink-faint truncate max-w-full' title={hint}>{hint}</span>
    </div>
  )
}

export default SettingControl
