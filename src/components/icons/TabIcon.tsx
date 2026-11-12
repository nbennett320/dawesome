import React from 'react'
import { IconProps, line16, round } from './types'

// a page with a tab on top: the effect's own interface in a tab of its own
export const TabIcon = (p: IconProps) => (
  <svg {...line16} {...round} {...p}>
    <path d='M2.5 13.5v-10a1 1 0 0 1 1-1h3.5l1 2h4.5a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1zM2.5 4.5h5' />
  </svg>
)
