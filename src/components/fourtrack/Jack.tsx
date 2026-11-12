import React from 'react'
import { cx } from '../../util/helpers'

export interface JackProps {
  // a plug sits in it, its cable leaving down and to the right
  plugged?: boolean
  size?: number
  className?: string
}

// a quarter-inch jack socket on the panel: a hex nut, its washer and the
// hole, and when plugged, a plug and a short length of cable. drawn in the
// theme's colours (fourtrack.scss)
export const Jack = ({ plugged = false, size = 36, className }: JackProps) => (
  <svg
    viewBox='0 0 40 40'
    width={size}
    height={size}
    aria-hidden
    data-plugged={plugged}
    className={cx('ft-jack', className)}
  >
    <polygon className='ft-jack__nut' points='28,16 22,26.4 10,26.4 4,16 10,5.6 22,5.6' />
    <circle className='ft-jack__washer' cx='16' cy='16' r='8' />
    <circle className='ft-jack__hole' cx='16' cy='16' r='4' />

    {plugged && (
      <g>
        <path className='ft-jack__cable' d='M27 29 C30 33 33 36 40 36.5' />
        {/* the strain relief, then the plug's body, angled toward us */}
        <line className='ft-jack__relief' x1='22' y1='23' x2='28' y2='30' />
        <line className='ft-jack__body' x1='16' y1='16' x2='22' y2='23' />
        <circle className='ft-jack__collar' cx='16' cy='16' r='5.5' />
      </g>
    )}
  </svg>
)
