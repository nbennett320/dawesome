import React from 'react'
import styles from './styles.module.scss'

interface Props {}

const TopbarBase = (props: React.PropsWithChildren<Props>) => (
  <nav className={`${styles.TopbarBase} w-full max-w-7xl mx-auto px-2 sm:px-6 lg:px-8 bg-gray-200`}>{props.children}</nav>
)

export default TopbarBase
