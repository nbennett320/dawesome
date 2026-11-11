import React from 'react'

// led: something to look at, danger: something that failed
const TONES = {
  led: 'border-amber-300 bg-amber-50 text-amber-900',
  danger: 'border-red-200 bg-red-50 text-red-900',
}

const Notice = ({ tone, role = 'alert', children, className = 'rounded border px-3 py-2' }: {
  tone: keyof typeof TONES
  role?: 'alert' | 'status'
  children: React.ReactNode
  // the box's shape and spacing; passing one replaces the default
  className?: string
}) => (
  <div role={role} className={`${className} ${TONES[tone]}`}>
    {children}
  </div>
)

export default Notice
