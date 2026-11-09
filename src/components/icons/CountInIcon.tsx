import React from 'react'

interface Props {}

// a bar of four beats, the first accented: counting in before recording
const CountInIcon = (props: React.SVGAttributes<Props>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    version="1.1"
    width="24"
    height="24"
    viewBox="0 0 24 24"
    { ...props }
  >
    <rect x="3" y="5" width="3" height="14" rx="1" />
    <rect x="8.5" y="10" width="3" height="9" rx="1" />
    <rect x="14" y="10" width="3" height="9" rx="1" />
    <rect x="19.5" y="10" width="3" height="9" rx="1" />
  </svg>
)

export default CountInIcon
