import React from 'react'
import {
  toggleSidebar,
  selectSidebar,
} from '../../state/slices/windowSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import SidebarBase from './SidebarBase'
import SidebarHeader from './SidebarHeader'
import SampleBrowser from '../toolbars/sample-browser/SampleBrowser'
import styles from './styles.module.scss'

interface Props {
}

const Sidebar = (props: React.PropsWithChildren<Props>) => {
  const open = useAppSelector(selectSidebar)
  const dispatch = useAppDispatch()

  return (
    <SidebarBase open={open}>
      <div className={`${styles.Sidebar} bg-gray-300`}>
        <div className={styles.SidebarHeaderContainer}>
          <SidebarHeader onClose={() => dispatch(toggleSidebar())} />
        </div>
        <div className={styles.SidebarBody}>
          <SampleBrowser />
        </div>
      </div>
    </SidebarBase>
  )
}

export default Sidebar
