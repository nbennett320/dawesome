import React from 'react'
import ButtonBase from './base/ButtonBase'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  type?: 'button' | 'reset' | 'submit',
}

const Button = (props: Props<HTMLButtonElement>) => (
  <ButtonBase
    {...props}
    className={styles.Button}
    type={props?.type ?? 'button'} 
  >
    {props.children} 
  </ButtonBase>
)

export default Button

