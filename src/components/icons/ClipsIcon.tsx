import React from 'react'
import { IconProps, filled, size24 } from './types'

// a clip: a card with a label band and a waveform in it, the sidebar's clips page
export const ClipsIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <rect x='3.5' y='6.5' width='17' height='11' rx='2' fill='none' stroke='currentColor' strokeWidth='2' />
    <path d='M3.5 10 H20.5' fill='none' stroke='currentColor' strokeWidth='2' />
    <path d='M6.5 14 l2 -1.5 l2 3 l2 -3 l2 3 l2 -1.5 h1' fill='none' stroke='currentColor' strokeWidth='2' />
  </svg>
)
