import React from 'react'
import { IconProps } from './types'

// a pane cut in two, for the tab bar: down the middle or across it
export const SplitIcon = ({ direction, ...p }: IconProps & { direction: 'vertical' | 'horizontal' }) => (
  <svg viewBox='0 0 16 16' fill='none' stroke='currentColor' strokeWidth='1.25' {...p}>
    <rect x='1.5' y='2.5' width='13' height='11' rx='1' />
    {direction === 'vertical'
      ? <line x1='8' y1='2.5' x2='8' y2='13.5' />
      : <line x1='1.5' y1='8' x2='14.5' y2='8' />}
  </svg>
)
