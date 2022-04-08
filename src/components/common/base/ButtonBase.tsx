import React from 'react'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  type: 'button' | 'reset' | 'submit'
}

const ButtonBase = (props: Props<HTMLButtonElement>) => (
  <button
    {...props}
    className={`input ${styles.ButtonBase}`}
    type={props?.type ?? 'button'}
  >
    {props.children}
  </button>
)

export default ButtonBase
