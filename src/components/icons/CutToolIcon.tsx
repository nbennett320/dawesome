import React from 'react'

interface Props {}

// a razor blade stood on end, with the notches of a double-edged blade and a
// slot down its middle
const CutToolIcon = (props: React.SVGAttributes<Props>) => (
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
    <path d="M8.5 3h7A1.5 1.5 0 0 1 17 4.5v6a1.5 1.5 0 0 0 0 3v6a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 7 19.5v-6a1.5 1.5 0 0 0 0-3v-6A1.5 1.5 0 0 1 8.5 3z" />
    <path d="M12 8v3M12 13v3" />
    <circle cx="12" cy="12" r="1" />
  </svg>
)

export default CutToolIcon
