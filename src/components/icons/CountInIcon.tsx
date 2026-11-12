import React from 'react'
import { IconProps, filled, size24 } from './types'

// a bar of four beats, the first accented: counting in before recording
export const CountInIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <rect x='3' y='5' width='3' height='14' rx='1' />
    <rect x='8.5' y='10' width='3' height='9' rx='1' />
    <rect x='14' y='10' width='3' height='9' rx='1' />
    <rect x='19.5' y='10' width='3' height='9' rx='1' />
  </svg>
)
