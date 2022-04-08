import React from 'react'
import styles from './styles.module.scss'

export interface InputI<T> extends React.HTMLProps<T> {}

const Input = (props: InputI<HTMLElement>) => (
  <input
    {...(props as React.HTMLProps<HTMLInputElement>)}
    className={`input ${styles.InputBase}`}
    type={props.type || 'text'}
  />
)

export default Input
