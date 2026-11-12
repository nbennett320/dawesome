import React from 'react'
import { IconProps, filled, size24 } from './types'

// a grid of steps, some lit: the sidebar's patterns page
export const PatternIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <rect x='3' y='4' width='4' height='4' rx='1' />
    <rect x='10' y='4' width='4' height='4' rx='1' opacity='.3' />
    <rect x='17' y='4' width='4' height='4' rx='1' />
    <rect x='3' y='10' width='4' height='4' rx='1' opacity='.3' />
    <rect x='10' y='10' width='4' height='4' rx='1' />
    <rect x='17' y='10' width='4' height='4' rx='1' opacity='.3' />
    <rect x='3' y='16' width='4' height='4' rx='1' />
    <rect x='10' y='16' width='4' height='4' rx='1' opacity='.3' />
    <rect x='17' y='16' width='4' height='4' rx='1' />
  </svg>
)
