import React from 'react'

interface Props {}

// notes on a piano roll, as short bars at different heights: the sidebar's
// note patterns page
const NotePatternIcon = (props: React.SVGAttributes<Props>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    { ...props }
  >
    <rect x="3" y="4" width="7" height="3.5" rx="1" />
    <rect x="12" y="9" width="5" height="3.5" rx="1" />
    <rect x="6" y="14" width="10" height="3.5" rx="1" opacity=".3" />
    <rect x="18" y="16.5" width="3" height="3.5" rx="1" />
  </svg>
)

export default NotePatternIcon
