import React from 'react'
import { IconProps, filled } from './types'

export const PauseIcon = (p: IconProps) => (
  <svg {...filled} {...p}><rect x='6' y='5' width='4' height='14' /><rect x='14' y='5' width='4' height='14' /></svg>
)
