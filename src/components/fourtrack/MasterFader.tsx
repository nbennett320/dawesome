import React from 'react'
import { cx, useControllable } from './util'

export interface MasterFaderProps {
  value?: number
  defaultValue?: number
  onChange?: (value: number) => void
  min?: number
  max?: number
  step?: number
  // the track's height in px
  height?: number
  // scale labels. every fifth of the range by default
  ticks?: number[]
  label?: string
  showReadout?: boolean
  className?: string
}

const CAP_H = 52

// a linear fader over a native range input (the mixer's faders use their own
// dB scale, see views/mixer/Fader)
export const MasterFader = ({
  value,
  defaultValue = 7.5,
  onChange,
  min = 0,
  max = 10,
  step = 0.1,
  height = 280,
  ticks,
  label = 'Master level',
  showReadout = true,
  className,
}: MasterFaderProps) => {
  const [v, setV] = useControllable(value, defaultValue, onChange)
  const travel = height - CAP_H
  const pos = (n: number) => (1 - (n - min) / (max - min)) * travel
  const scale = ticks ?? Array.from({ length: 6 }, (_, i) => max - i * ((max - min) / 5))

  return (
    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10 }}>
      <div className={cx('ft-fader', className)} style={{ height }}>
        {scale.map(t => (
          <React.Fragment key={t}>
            <span className='ft-fader__tick' style={{ top: pos(t) + CAP_H / 2 - 7 }}>{+t.toFixed(1)}</span>
            <span className='ft-fader__line' style={{ top: pos(t) + CAP_H / 2 }} />
          </React.Fragment>
        ))}
        <div className='ft-fader__slot' />
        <input
          className='ft-fader__input'
          type='range'
          min={min}
          max={max}
          step={step}
          value={v}
          aria-label={label}
          aria-orientation='vertical'
          onChange={e => setV(parseFloat(e.target.value))}
        />
        <div className='ft-fader__cap' style={{ top: pos(v) }} />
      </div>
      {showReadout && <span className='ft-readout'>{v.toFixed(1)}</span>}
    </div>
  )
}
