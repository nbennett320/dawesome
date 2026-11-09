import React from 'react'

interface Props {}

// a grid of steps, some lit: the sidebar's patterns page
const PatternIcon = (props: React.SVGAttributes<Props>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    { ...props }
  >
    <rect x="3" y="4" width="4" height="4" rx="1" />
    <rect x="10" y="4" width="4" height="4" rx="1" opacity=".3" />
    <rect x="17" y="4" width="4" height="4" rx="1" />
    <rect x="3" y="10" width="4" height="4" rx="1" opacity=".3" />
    <rect x="10" y="10" width="4" height="4" rx="1" />
    <rect x="17" y="10" width="4" height="4" rx="1" opacity=".3" />
    <rect x="3" y="16" width="4" height="4" rx="1" />
    <rect x="10" y="16" width="4" height="4" rx="1" opacity=".3" />
    <rect x="17" y="16" width="4" height="4" rx="1" />
  </svg>
)

export default PatternIcon
