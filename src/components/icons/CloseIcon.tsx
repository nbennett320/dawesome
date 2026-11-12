import React from 'react'
import { IconProps, line16, round } from './types'

// a cross: close a tab, take an effect off its channel
export const CloseIcon = (p: IconProps) => (
  <svg {...line16} {...round} {...p}>
    <path d='M4 4l8 8M12 4l-8 8' />
  </svg>
)
