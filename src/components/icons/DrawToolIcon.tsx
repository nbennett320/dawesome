import React from 'react'

interface Props {}

// a pencil, point down to the left
const DrawToolIcon = (props: React.SVGAttributes<Props>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    fill="none"
    stroke="currentColor"
    strokeWidth={1.75}
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M4 20l1.2-4.6L15.8 4.8a1.7 1.7 0 0 1 2.4 0l1 1a1.7 1.7 0 0 1 0 2.4L8.6 18.8z" />
    <path d="M14 6.6l3.4 3.4" />
  </svg>
)

export default DrawToolIcon
