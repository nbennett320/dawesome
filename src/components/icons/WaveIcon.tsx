import React from 'react'
import { IconProps, round } from './types'

// a little waveform, for a sample
export const WaveIcon = ({ className = 'w-3.5 h-3.5 shrink-0 text-ink-faint', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={className} aria-hidden {...p}>
    <path d='M2 8h1.5M5 5v6M7.5 3v10M10 6v4M12.5 7v2M14 8h0' fill='none' stroke='currentColor' strokeWidth='1.5' {...round} />
  </svg>
)
