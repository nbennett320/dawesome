import React from 'react'
import Button from '../common/Button'
import SidebarIcon from '../icons/SidebarIcon'
import { SidebarPage } from './Sidebar'
import styles from './styles.module.scss'

interface Props {
  open: boolean
  page: SidebarPage
  onChangePage: (next: SidebarPage) => void
  onClose: () => void
}

const SidebarList = (props: React.PropsWithChildren<Props>) => {
  const handleClick = (page: SidebarPage) => {
    if(props.open && props.page === page) {
      props.onClose()
    } else if(props.open) {
      props.onChangePage(page)
    } else {
      props.onClose()
    }
  }

  return (
    <div className="flex flex-col bg-slate-400 px-1">
      <Button 
        className="z-10"
        onClick={() => { handleClick(SidebarPage.FileBrowser) }} 
      >
        <SidebarIcon fill="white" />
      </Button>
    </div>
  )
}

export default SidebarList
