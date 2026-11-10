import React from 'react'

interface Props {}

const SelectToolIcon = (props: React.SVGAttributes<Props>) => (
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
    <path d="M6 3l12 9-5.2 1.3 3 6.2-2.4 1.1-3-6.2L6 18z" />
  </svg>
)

export default SelectToolIcon
