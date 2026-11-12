import React from 'react'
import { IconProps, line16, round } from './types'

// a box with an arrow leaving it: the effect's interface in a window of its own
export const PopOutIcon = (p: IconProps) => (
  <svg {...line16} {...round} {...p}>
    <path d='M7 3.5H3.5a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V9' />
    <path d='M9.5 2.5h4v4M13.5 2.5L7.5 8.5' />
  </svg>
)
