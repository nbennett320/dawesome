import React from 'react'
import { IconProps } from './types'

export const ResetIcon = (p: IconProps) => (
  <svg
    viewBox='0 0 24 24'
    fill='none'
    stroke='currentColor'
    strokeWidth={2}
    strokeLinecap='round'
    strokeLinejoin='round'
    aria-hidden
    {...p}
  >
    <path d='M3 12a9 9 0 1 0 3-6.7' /><path d='M3 4v5h5' />
  </svg>
)
