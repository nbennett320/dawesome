import React from 'react'
import { cx } from '../../util/helpers'

export type LedState = 'off' | 'on' | 'dim' | 'blink'
export type LedColor = 'red' | 'amber' | 'lime' | 'blue'

export interface LedProps extends React.HTMLAttributes<HTMLSpanElement> {
  color?: LedColor
  state?: LedState
}

export const Led = ({ color = 'red', state = 'off', className, ...rest }: LedProps) => (
  <span
    aria-hidden
    data-state={state}
    className={cx('ft-led', `ft-led--${color}`, className)}
    {...rest}
  />
)
