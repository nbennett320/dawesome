import React from 'react'

// props for the transport icons. they take the colour of the text around them
export type IconProps = React.SVGProps<SVGSVGElement>

// shared by the filled icons
export const filled = { viewBox: '0 0 24 24', fill: 'currentColor', 'aria-hidden': true } as const
