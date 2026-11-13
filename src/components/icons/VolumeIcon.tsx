import React from 'react'
import { IconProps, filled } from './types'

// a speaker: loudness (a volume knob's cap)
export const VolumeIcon = (p: IconProps) => (
  <svg {...filled} {...p}><path d='M3 9h4.5L13 4.5v15L7.5 15H3z' /><path d='M16 8.5a5 5 0 0 1 0 7' fill='none' stroke='currentColor' strokeWidth='2.5' strokeLinecap='round' /></svg>
)
