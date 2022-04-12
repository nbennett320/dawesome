import React from 'react'
import ButtonBase from './base/ButtonBase'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  type?: 'button' | 'reset' | 'submit'
}

const Button = (props: Props<HTMLButtonElement>) => (
  <ButtonBase
    {...props}
    type={props?.type ?? 'button'}
  >
    <div className={`${styles.Button} ${props?.className ? props.className : ''}`}>
      {props.children}
    </div>
  </ButtonBase>
)

export default Button
