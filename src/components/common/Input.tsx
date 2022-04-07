import React from 'react'
import InputBase from './base/InputBase'
import styles from './styles.module.scss'

export interface InputI<T> extends React.HTMLProps<T> {}

const Input = (props: InputI<HTMLInputElement>) => (
  <div className={`${styles.Input} ${props.className}`}>
    <InputBase 
      {...props as React.HTMLProps<HTMLInputElement>} 
      type={props?.type || 'text'} 
    />
  </div>
)

export default Input
