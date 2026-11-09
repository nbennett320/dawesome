import React from 'react'
import { IconProps, filled } from './types'

export const RecordIcon = (p: IconProps) => (
  <svg {...filled} {...p}><circle cx='12' cy='12' r='6' /></svg>
)
