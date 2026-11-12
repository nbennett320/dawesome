import React from 'react'
import { cx } from '../../util/helpers'

export type LedState = 'off' | 'on' | 'dim' | 'blink'
export type LedColor = 'red' | 'amber' | 'lime' | 'blue'

export interface LedProps extends React.HTMLAttributes<HTMLSpanElement> {
  color?: LedColor
  state?: LedState
}

// forwards its ref, for an LED lit straight from a loop (see useMixerMeters)
export const Led = React.forwardRef<HTMLSpanElement, LedProps>(
  ({ color = 'red', state = 'off', className, ...rest }, ref) => (
    <span
      ref={ref}
      aria-hidden
      data-state={state}
      className={cx('ft-led', `ft-led--${color}`, className)}
      {...rest}
    />
  ),
)
Led.displayName = 'Led'
