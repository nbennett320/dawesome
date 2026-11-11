import React from 'react'

const FILLS = { ok: 'bg-lime-500', clip: 'bg-red-500' }

// a track and its fill, for progress and levels. the track's size and the aria
// attributes come from the caller
const Bar = ({ value, tone = 'ok', className = '', ...aria }: {
  // 0 to 1
  value: number
  tone?: keyof typeof FILLS
  className?: string
} & React.HTMLAttributes<HTMLDivElement>) => (
  <div className={`h-2 rounded-sm bg-slate-200 overflow-hidden ${className}`} {...aria}>
    <div className={`h-full ${FILLS[tone]}`} style={{ width: `${value * 100}%` }} />
  </div>
)

export default Bar
