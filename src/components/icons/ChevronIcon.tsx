import React from 'react'
import { IconProps, round } from './types'

// points right, or down when open
export const ChevronIcon = ({ open, className = '', ...p }: IconProps & { open: boolean }) => (
  <svg
    viewBox='0 0 16 16'
    className={`w-3 h-3 shrink-0 text-ink-faint transition-transform ${open ? 'rotate-90' : ''} ${className}`}
    aria-hidden
    {...p}
  >
    <path d='M6 4l4 4-4 4' fill='none' stroke='currentColor' strokeWidth='1.75' {...round} />
  </svg>
)
