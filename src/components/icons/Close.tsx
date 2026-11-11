import React from 'react'

interface Props extends React.SVGAttributes<SVGSVGElement> {
  fill?: string
}

const Close = ({ fill = 'currentColor', ...props }: Props) => (
  <svg 
    xmlns="http://www.w3.org/2000/svg" 
    viewBox="0 0 24 24" 
    width="24" 
    height="24"
    { ...props }
  >
    <path 
      fill="none" 
      d="M0 0h24v24H0z"
    />
    <path 
      d="M12 10.586l4.95-4.95 1.414 1.414-4.95 4.95 4.95 4.95-1.414 1.414-4.95-4.95-4.95 4.95-1.414-1.414 4.95-4.95-4.95-4.95L7.05 5.636z" 
      fill={fill}
    />
  </svg>
)

export default Close
