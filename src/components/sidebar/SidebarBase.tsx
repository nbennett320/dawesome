import React from 'react'
import styles from './styles.module.scss'

interface Props {
  open: boolean
  onOpen?: () => void
  onClose?: () => void
}

const SidebarBase = (props: React.PropsWithChildren<Props>) => {
  React.useEffect(() => {
    if(props.open) {
      if(props.onOpen) {
        props.onOpen()
      }
    } else if(!props.open && props.onClose) {
      props.onClose()
    }
  }, [props, props.open])

  return (
    <div className={`${styles.SidebarBase}`}>
      {props.children}
    </div>
  )
}

export default SidebarBase
