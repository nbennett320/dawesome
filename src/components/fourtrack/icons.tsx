import React from 'react'

// transport icons. they take the colour of the text around them

type IconProps = React.SVGProps<SVGSVGElement>

const base = { viewBox: '0 0 24 24', fill: 'currentColor', 'aria-hidden': true } as const

export const RecordIcon = (p: IconProps) => (
  <svg {...base} {...p}><circle cx='12' cy='12' r='6' /></svg>
)
export const PlayIcon = (p: IconProps) => (
  <svg {...base} {...p}><path d='M8 5v14l11-7z' /></svg>
)
export const RewindIcon = (p: IconProps) => (
  <svg {...base} {...p}><path d='M11.5 6v12L3 12zM21 6v12l-8.5-6z' /></svg>
)
export const FastForwardIcon = (p: IconProps) => (
  <svg {...base} {...p}><path d='M12.5 6v12L21 12zM3 6v12l8.5-6z' /></svg>
)
export const StopIcon = (p: IconProps) => (
  <svg {...base} {...p}><rect x='6' y='6' width='12' height='12' rx='1' /></svg>
)
export const PauseIcon = (p: IconProps) => (
  <svg {...base} {...p}><rect x='6' y='5' width='4' height='14' /><rect x='14' y='5' width='4' height='14' /></svg>
)
export const ResetIcon = (p: IconProps) => (
  <svg
    viewBox='0 0 24 24'
    fill='none'
    stroke='currentColor'
    strokeWidth={2}
    strokeLinecap='round'
    strokeLinejoin='round'
    aria-hidden
    {...p}
  >
    <path d='M3 12a9 9 0 1 0 3-6.7' /><path d='M3 4v5h5' />
  </svg>
)
