import React from 'react'

// icons for the Effects Rack's rows and its strip. they take the colour of
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

// corners pointing out: show the effect's own interface in the tab
export const ExpandIcon = () => (
  <svg {...stroke}>
    <path d='M9.5 2.5h4v4M6.5 13.5h-4v-4M13.5 2.5L9 7M2.5 13.5L7 9' />
  </svg>
)

// corners pointing in: back to the stack
export const CollapseIcon = () => (
  <svg {...stroke}>
    <path d='M13.5 6.5h-4v-4M2.5 9.5h4v4M9.5 6.5L14 2M6.5 9.5L2 14' />
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

// the previous or next effect
export const StepIcon = ({ direction }: { direction: 'prev' | 'next' }) => (
  <svg {...stroke}>
    <path d={direction === 'prev' ? 'M10 3.5L5.5 8l4.5 4.5' : 'M6 3.5L10.5 8 6 12.5'} />
  </svg>
)
