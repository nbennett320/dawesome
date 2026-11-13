import React from 'react'
import { IconProps, line, round } from './types'

// arrows to the left and right: where a sound sits between the speakers (a
// pan knob's cap)
export const PanIcon = (p: IconProps) => (
  <svg {...line} {...round} strokeWidth={3} {...p}><path d='M3 12h18M7 7.5 2.5 12 7 16.5M17 7.5l4.5 4.5-4.5 4.5' /></svg>
)
