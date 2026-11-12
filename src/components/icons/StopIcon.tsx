import React from 'react'
import { IconProps, filled } from './types'

export const StopIcon = (p: IconProps) => (
  <svg {...filled} {...p}><rect x='6' y='6' width='12' height='12' rx='1' /></svg>
)
