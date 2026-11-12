import React from 'react'
import { IconProps } from './types'

// a few piano keys, for an instrument plugin
export const KeysIcon = ({ className = 'w-3.5 h-3.5 text-ink-faint', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={`shrink-0 ${className}`} aria-hidden {...p}>
    <rect x='1.5' y='3' width='13' height='10' rx='1' fill='currentColor' opacity='.2' stroke='currentColor' strokeWidth='1' />
    <path d='M5.5 3v10M10.5 3v10' fill='none' stroke='currentColor' strokeWidth='1' />
    <path d='M4.5 3h2v5.5h-2zM9.5 3h2v5.5h-2z' fill='currentColor' />
  </svg>
)
