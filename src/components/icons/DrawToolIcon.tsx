import React from 'react'
import { IconProps, line, round, size24 } from './types'

// a pencil, point down to the left
export const DrawToolIcon = (p: IconProps) => (
  <svg {...line} {...round} {...size24} {...p}>
    <path d='M4 20l1.2-4.6L15.8 4.8a1.7 1.7 0 0 1 2.4 0l1 1a1.7 1.7 0 0 1 0 2.4L8.6 18.8z' />
    <path d='M14 6.6l3.4 3.4' />
  </svg>
)
