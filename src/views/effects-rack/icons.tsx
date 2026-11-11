import React from 'react'

// icons for the Effects Rack's rows and a plugin's tab. they take the colour of
// the text around them

const stroke = {
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.5,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
  'aria-hidden': true,
} as const

// two columns of dots: a row can be dragged by it
export const GripIcon = ({ className = '' }: { className?: string }) => (
  <svg viewBox='0 0 16 16' className={`w-3 h-3.5 shrink-0 ${className}`} fill='currentColor' aria-hidden>
    <circle cx='6' cy='4' r='1.1' />
    <circle cx='10' cy='4' r='1.1' />
    <circle cx='6' cy='8' r='1.1' />
    <circle cx='10' cy='8' r='1.1' />
    <circle cx='6' cy='12' r='1.1' />
    <circle cx='10' cy='12' r='1.1' />
  </svg>
)

// a page with a tab on top: the effect's own interface in a tab of its own
export const TabIcon = () => (
  <svg {...stroke}>
    <path d='M2.5 13.5v-10a1 1 0 0 1 1-1h3.5l1 2h4.5a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1zM2.5 4.5h5' />
  </svg>
)

// a box with an arrow leaving it: the effect's interface in a window of its own
export const PopOutIcon = () => (
  <svg {...stroke}>
    <path d='M7 3.5H3.5a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V9' />
    <path d='M9.5 2.5h4v4M13.5 2.5L7.5 8.5' />
  </svg>
)

// a cross: take the effect off the channel
export const RemoveIcon = () => (
  <svg {...stroke}>
    <path d='M4 4l8 8M12 4l-8 8' />
  </svg>
)
