import React from 'react'

interface Props {}

// a record dot in a ring: the sidebar's recordings page
const RecordingsIcon = (props: React.SVGAttributes<Props>) => (
  <svg
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    { ...props }
  >
    <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" strokeWidth="2" />
    <circle cx="12" cy="12" r="4.5" />
  </svg>
)

export default RecordingsIcon
