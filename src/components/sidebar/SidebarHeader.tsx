import React from 'react'
import Button from '../common/Button'
import SidebarIcon from '../icons/SidebarIcon'
import styles from './styles.module.scss'

interface Props {
  onClose: () => void
}

const SidebarHeader = (props: React.PropsWithChildren<Props>) => (
  <div className={`${styles.SidebarHeader}`}>
    <Button onClick={props.onClose} className="z-10">
      <SidebarIcon fill="white" />
    </Button>
  </div>
)

export default SidebarHeader
