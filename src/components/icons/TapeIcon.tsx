import React from 'react'
import { IconProps, line } from './types'

// a cassette
export const TapeIcon = (p: IconProps) => (
  <svg {...line} {...p}>
    <rect x='3' y='6' width='18' height='12' rx='2' />
    <circle cx='9' cy='12' r='1.75' />
    <circle cx='15' cy='12' r='1.75' />
  </svg>
)
