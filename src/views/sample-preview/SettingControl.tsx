import React from 'react'

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
  // a short description of the value, shown beside the label
  hint?: string
  onChange: (value: number) => void
  // double clicking the label or slider calls this
  onReset: () => void
}

// round away floating point noise for display
const tidy = (n: number) => Number(n.toFixed(4))

// a setting with a slider and a number field. double click the label or
// slider to reset it
const SettingControl = ({
  label,
  value,
  min,
  max,
  step,
  unit,
  fieldScale = 1,
  hint,
  onChange,
  onReset,
}: Props) => {
  // what's typed in the number field, applied on enter or blur
  const [draft, setDraft] = React.useState<string | null>(null)

  const commit = () => {
    if(draft === null) return

    const typed = parseFloat(draft)
    if(Number.isFinite(typed)) onChange(typed / fieldScale)
    setDraft(null)
  }

  return (
    <div className='flex flex-col gap-1 min-w-0'>
      <div className='flex flex-row items-baseline justify-between gap-2 text-xs'>
        <span
          className='text-gray-400 select-none'
          title='Double-click to reset'
          onDoubleClick={onReset}
        >
          {label}
        </span>
        {hint && <span className='text-gray-500 truncate'>{hint}</span>}
      </div>

      <div className='flex flex-row items-center gap-2'>
        <input
          type='range'
          aria-label={label}
          className='flex-1 min-w-0'
          min={min}
          max={max}
          step={step}
          value={value}
          title='Double-click to reset'
          onChange={e => onChange(parseFloat(e.currentTarget.value))}
          onDoubleClick={onReset}
        />

        <div className='flex flex-row items-center gap-1'>
          <input
            type='number'
            className='w-16 text-gray-600 text-sm rounded border border-gray-300 px-1'
            min={min * fieldScale}
            max={max * fieldScale}
            step={step * fieldScale}
            value={draft ?? tidy(value * fieldScale)}
            aria-label={`${label} value`}
            onChange={e => setDraft(e.currentTarget.value)}
            onBlur={commit}
            onKeyDown={e => {
              if(e.key === 'Enter') commit()
              if(e.key === 'Escape') setDraft(null)
            }}
          />
          {unit && <span className='w-6 text-xs text-gray-500'>{unit}</span>}
        </div>
      </div>
    </div>
  )
}

export default SettingControl
