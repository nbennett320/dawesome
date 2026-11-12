import React from 'react'
import { IconProps, round } from './types'

// a knob, for an effect plugin
export const KnobIcon = ({ className = 'w-3.5 h-3.5 shrink-0 text-ink-faint', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={className} aria-hidden {...p}>
    <circle cx='8' cy='8' r='5.25' fill='currentColor' opacity='.35' stroke='currentColor' strokeWidth='1' />
    <path d='M8 8L5.5 5.5' fill='none' stroke='currentColor' strokeWidth='1.5' {...round} />
  </svg>
)
