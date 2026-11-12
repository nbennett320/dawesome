import React from 'react'
import { IconProps, filled, size24 } from './types'

// two units of a rack, each with a knob and a light: the sidebar's effects page
export const EffectsIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <rect x='3' y='4' width='18' height='7' rx='1.5' fill='none' stroke='currentColor' strokeWidth='2' />
    <rect x='3' y='13' width='18' height='7' rx='1.5' fill='none' stroke='currentColor' strokeWidth='2' />
    <circle cx='8' cy='7.5' r='1.75' />
    <circle cx='14' cy='16.5' r='1.75' />
    <rect x='15.5' y='6.75' width='3' height='1.5' rx='.75' opacity='.5' />
    <rect x='5.5' y='15.75' width='3' height='1.5' rx='.75' opacity='.5' />
  </svg>
)
