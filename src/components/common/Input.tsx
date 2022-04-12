import React from 'react'
import InputBase from './base/InputBase'
import styles from './styles.module.scss'

export interface InputI<T> extends React.HTMLProps<T> {
  label?: string
  labelPosition?: 'top' | 'bottom'
}

const Input = (props: InputI<HTMLInputElement>) => (
  <div className={`${styles.Input}`}>
    {props?.label 
      && props.labelPosition !== 'bottom' 
      && <span className="text-gray-400 text-xs text-left">
        {props.label}
      </span>
    }
    <InputBase
      {...(props as React.HTMLProps<HTMLInputElement>)}
      type={props?.type || 'text'}
      className={`${props.className}`}
    />
  </div>
)

export default Input
