import React from 'react'
import { IconProps } from './types'

// the outline of a folder
export const FOLDER_OUTLINE = 'M1.5 4.5a1 1 0 0 1 1-1h3.6l1.4 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z'

export const FolderIcon = ({ className = 'w-3.5 h-3.5 shrink-0 text-ink-faint', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={className} aria-hidden {...p}>
    <path d={FOLDER_OUTLINE} fill='currentColor' opacity='.35' stroke='currentColor' strokeWidth='1' />
  </svg>
)
