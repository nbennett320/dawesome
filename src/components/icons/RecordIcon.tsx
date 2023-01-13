import React from 'react'

interface Props {}

const RecordIcon = (props: React.SVGAttributes<Props>) => (
  <svg 
    xmlns="http://www.w3.org/2000/svg"
    viewBox="0 0 24 24"
    width="24"
    height="24"
    { ...props }
  >
    <circle cx="12" cy="12" r="6" />
  </svg>
)

export default RecordIcon

