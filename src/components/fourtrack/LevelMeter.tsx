import React from 'react'
import { Led, LedColor } from './Led'

export interface LevelMeterProps {
  // 0 to 1
  level: number
  segments?: Array<{ label: string, threshold: number, color: LedColor }>
}

const DEFAULT_SEGMENTS: NonNullable<LevelMeterProps['segments']> = [
  { label: '-10', threshold: 0.28, color: 'lime' },
  { label: '-5', threshold: 0.5, color: 'amber' },
  { label: '0', threshold: 0.72, color: 'red' },
  { label: '+6', threshold: 0.9, color: 'red' },
]

// a row of LEDs that light as the level rises
export const LevelMeter = ({ level, segments = DEFAULT_SEGMENTS }: LevelMeterProps) => (
  <div className='ft-meter' role='meter' aria-label='Level' aria-valuemin={0} aria-valuemax={1} aria-valuenow={level}>
    {segments.map(s => (
      <div key={s.label} className='ft-meter__cell'>
        <Led color={s.color} state={level > s.threshold ? 'on' : 'off'} />
        <span className='ft-label'>{s.label}</span>
      </div>
    ))}
  </div>
)
