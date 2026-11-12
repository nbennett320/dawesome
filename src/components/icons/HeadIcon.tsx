import React from 'react'
import { IconProps, line } from './types'

// the tape running over a play head
export const HeadIcon = (p: IconProps) => (
  <svg {...line} {...p}>
    <path d='M3 9h18' />
    <rect x='8' y='9' width='8' height='7' rx='1.5' />
  </svg>
)
