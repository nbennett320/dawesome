import React from 'react'
import { IconProps, round } from './types'

// a warning sign, for a plugin that couldn't be read
export const WarningIcon = ({ className = 'w-3.5 h-3.5 shrink-0 text-danger', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={className} aria-hidden {...p}>
    <path d='M8 2.5l6 10.5H2z' fill='none' stroke='currentColor' strokeWidth='1.25' strokeLinejoin='round' />
    <path d='M8 6.5v3M8 11.25v.01' fill='none' stroke='currentColor' strokeWidth='1.5' {...round} />
  </svg>
)
