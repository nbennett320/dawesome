import React from 'react'
import { IconProps, filled, size24 } from './types'

// a record dot in a ring: the sidebar's recordings page
export const RecordingsIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <circle cx='12' cy='12' r='8.5' fill='none' stroke='currentColor' strokeWidth='2' />
    <circle cx='12' cy='12' r='4.5' />
  </svg>
)
