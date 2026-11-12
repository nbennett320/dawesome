import React from 'react'
import { FOLDER_OUTLINE } from './FolderIcon'
import { IconProps, round } from './types'

// a folder with a plus, for making a new one
export const NewFolderIcon = ({ className = 'w-3.5 h-3.5', ...p }: IconProps) => (
  <svg viewBox='0 0 16 16' className={className} aria-hidden {...p}>
    <path d={FOLDER_OUTLINE} fill='none' stroke='currentColor' strokeWidth='1.25' />
    <path d='M8 7.5v4M6 9.5h4' fill='none' stroke='currentColor' strokeWidth='1.25' {...round} />
  </svg>
)
