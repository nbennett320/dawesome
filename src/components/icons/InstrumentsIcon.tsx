import React from 'react'

interface Props {}

// an octave of piano keys: the sidebar's instruments page
const InstrumentsIcon = (props: React.SVGAttributes<Props>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    { ...props }
  >
    <rect x="3" y="4" width="18" height="16" rx="1.5" fill="none" stroke="currentColor" strokeWidth="2" />
    <path d="M9 12v8M15 12v8" fill="none" stroke="currentColor" strokeWidth="1.5" />
    <rect x="7.5" y="4" width="3" height="8" rx=".5" />
    <rect x="13.5" y="4" width="3" height="8" rx=".5" />
  </svg>
)

export default InstrumentsIcon
