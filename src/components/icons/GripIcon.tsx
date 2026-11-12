import React from 'react'
import { IconProps } from './types'

// two columns of dots: a row can be dragged by it
export const GripIcon = ({ className = '', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={`w-3 h-3.5 shrink-0 ${className}`} fill='currentColor' aria-hidden {...p}>
    <circle cx='6' cy='4' r='1.1' />
    <circle cx='10' cy='4' r='1.1' />
    <circle cx='6' cy='8' r='1.1' />
    <circle cx='10' cy='8' r='1.1' />
    <circle cx='6' cy='12' r='1.1' />
    <circle cx='10' cy='12' r='1.1' />
  </svg>
)
