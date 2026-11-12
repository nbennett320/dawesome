import React from 'react'

// every icon is an svg that takes the props of one, and the colour of the
// text around it
export type IconProps = React.SVGProps<SVGSVGElement>

// 24 by 24 glyphs, filled
export const filled = { viewBox: '0 0 24 24', fill: 'currentColor', 'aria-hidden': true } as const

// 24 by 24 glyphs, drawn with a line
export const line = { viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', strokeWidth: 1.75, 'aria-hidden': true } as const

// 16 by 16 glyphs, drawn with a line
export const line16 = { viewBox: '0 0 16 16', fill: 'none', stroke: 'currentColor', strokeWidth: 1.5, 'aria-hidden': true } as const

// round ends and corners, for a line
export const round = { strokeLinecap: 'round', strokeLinejoin: 'round' } as const

// a fixed size. without it the icon is as big as the element around it makes it
export const size24 = { width: 24, height: 24 } as const
