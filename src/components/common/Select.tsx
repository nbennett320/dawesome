import React from 'react'
import SelectBase from './base/SelectBase'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  value?: string | number
  label?: string
}

const Select = (props: React.PropsWithChildren<Props<HTMLSelectElement>>) => (
  <div className="mt-2">
    {props?.label && <div className="text-gray-400 text-xs">
      {props.label}
    </div>}
    <SelectBase {...props}>
      <div className={`${styles.Select} w-64`}>
        {props.children}
      </div>
    </SelectBase>
  </div>
)

export default Select
