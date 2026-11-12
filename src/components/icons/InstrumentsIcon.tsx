import React from 'react'
import { IconProps, filled, size24 } from './types'

// an octave of piano keys: the sidebar's instruments page
export const InstrumentsIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <rect x='3' y='4' width='18' height='16' rx='1.5' fill='none' stroke='currentColor' strokeWidth='2' />
    <path d='M9 12v8M15 12v8' fill='none' stroke='currentColor' strokeWidth='1.5' />
    <rect x='7.5' y='4' width='3' height='8' rx='.5' />
    <rect x='13.5' y='4' width='3' height='8' rx='.5' />
  </svg>
)
