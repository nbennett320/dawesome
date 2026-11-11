import React from 'react'

// icons for the rows of the sidebar's browsers

// points right, or down when open
export const Chevron = ({ open }: { open: boolean }) => (
  <svg
    viewBox='0 0 16 16'
    className={`w-3 h-3 shrink-0 text-ink-faint transition-transform ${open ? 'rotate-90' : ''}`}
    aria-hidden
  >
    <path d='M6 4l4 4-4 4' fill='none' stroke='currentColor' strokeWidth='1.75' strokeLinecap='round' strokeLinejoin='round' />
  </svg>
)

export const FolderIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5 shrink-0 text-ink-faint' aria-hidden>
    <path d='M1.5 4.5a1 1 0 0 1 1-1h3.6l1.4 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z' fill='currentColor' opacity='.35' stroke='currentColor' strokeWidth='1' />
  </svg>
)

// a little waveform, for a sample
export const WaveIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5 shrink-0 text-ink-faint' aria-hidden>
    <path d='M2 8h1.5M5 5v6M7.5 3v10M10 6v4M12.5 7v2M14 8h0' fill='none' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
  </svg>
)

// a folder with a plus, for making a new one
export const NewFolderIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5' aria-hidden>
    <path d='M1.5 4.5a1 1 0 0 1 1-1h3.6l1.4 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z' fill='none' stroke='currentColor' strokeWidth='1.25' />
    <path d='M8 7.5v4M6 9.5h4' fill='none' stroke='currentColor' strokeWidth='1.25' strokeLinecap='round' />
  </svg>
)

// a knob, for an effect plugin
export const KnobIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5 shrink-0 text-ink-faint' aria-hidden>
    <circle cx='8' cy='8' r='5.25' fill='currentColor' opacity='.35' stroke='currentColor' strokeWidth='1' />
    <path d='M8 8L5.5 5.5' fill='none' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
  </svg>
)

// a warning sign, for a plugin that couldn't be read
export const WarningIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5 shrink-0 text-danger' aria-hidden>
    <path d='M8 2.5l6 10.5H2z' fill='none' stroke='currentColor' strokeWidth='1.25' strokeLinejoin='round' />
    <path d='M8 6.5v3M8 11.25v.01' fill='none' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
  </svg>
)
