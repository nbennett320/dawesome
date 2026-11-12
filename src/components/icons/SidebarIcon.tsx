import React from 'react'
import { IconProps, filled, size24 } from './types'

export const SidebarIcon = (p: IconProps) => (
  <svg {...filled} {...size24} {...p}>
    <path d='M3 3h18a1 1 0 0 1 1 1v16a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zm5 2H4v14h4V5zm2 0v14h10V5H10z' />
  </svg>
)
