import React from 'react'
import { cx } from './util'

export type AnalogButtonSize = 'xs' | 'sm' | 'md' | 'lg'

export interface AnalogButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, 'onClick'> {
  // an icon (or short text), centred on the cap
  children?: React.ReactNode
  size?: AnalogButtonSize
  // held down like a latching key. leave it out for momentary keys
  latched?: boolean
  // a short text cap (e.g. MUTE) instead of an icon
  variant?: 'icon' | 'text'
  onPress?: (e: React.MouseEvent<HTMLButtonElement>) => void
  className?: string
  type?: 'button' | 'submit' | 'reset'
}

export const AnalogButton = React.forwardRef<HTMLButtonElement, AnalogButtonProps>(
  ({ children, size = 'md', latched, variant = 'icon', onPress, className, type = 'button', ...rest }, ref) => (
    <button
      ref={ref}
      // eslint-disable-next-line react/button-has-type
      type={type}
      aria-pressed={latched === undefined ? undefined : latched}
      className={cx(
        'ft-btn',
        size !== 'md' && `ft-btn--${size}`,
        variant === 'text' && 'ft-btn--key',
        className,
      )}
      onClick={onPress}
      {...rest}
    >
      {variant === 'text' ? children : <span className='ft-btn__icon'>{children}</span>}
    </button>
  ),
)
AnalogButton.displayName = 'AnalogButton'
