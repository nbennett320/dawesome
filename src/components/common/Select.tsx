import React from 'react'
import SelectBase from './base/SelectBase'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  value?: string | number
}

const Select = (props: React.PropsWithChildren<Props<HTMLSelectElement>>) => (
  <SelectBase {...props}>
    <div className={`${styles.Select} w-64`}>
      {props.children}
    </div>
  </SelectBase>
)

export default Select
