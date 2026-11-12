import React from 'react'
import { IconProps, line, round, size24 } from './types'

export const SelectToolIcon = (p: IconProps) => (
  <svg {...line} {...round} {...size24} {...p}>
    <path d='M6 3l12 9-5.2 1.3 3 6.2-2.4 1.1-3-6.2L6 18z' />
  </svg>
)
