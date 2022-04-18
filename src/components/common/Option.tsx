import React from 'react'
import styles from './styles.module.scss'

export interface Props<T> extends React.HTMLProps<T> {
  selected?: boolean
}

const Option = (props: React.PropsWithChildren<Props<HTMLLIElement>>) => (
  <li 
    id={props.id}
    role="option" 
    aria-selected={props.selected}
    className={`${styles.Option} text-gray-700 hover:bg-blue-400 hover:text-white select-none text-sm relative py-2 pl-3 pr-9`}
    {...props}
  >
    {props.children}
  </li>
)

export default Option
