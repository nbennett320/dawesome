import React from 'react'
import {
  toggleSidebar,
  selectSidebar,
} from '../../state/slices/windowSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import SidebarBase from './SidebarBase'
import SidebarList from './SidebarList'
import SampleBrowser from '../toolbars/sample-browser/SampleBrowser'
import styles from './styles.module.scss'

interface Props {
}

export enum SidebarPage {
  FileBrowser
}

const Sidebar = (props: React.PropsWithChildren<Props>) => {
  const open = useAppSelector(selectSidebar)
  const dispatch = useAppDispatch()
  const [page, setPage] = React.useState<SidebarPage>(SidebarPage.FileBrowser)

  const handleChangePage = (next: SidebarPage) => {
    setPage(next)
  }

  return (
    <SidebarBase open={open}>
      <div className={`${styles.Sidebar} flex flex-row bg-gray-300`}>
        <div className={styles.SidebarListContainer}>
          <SidebarList 
            open={open}
            page={page}
            onChangePage={handleChangePage}
            onClose={() => dispatch(toggleSidebar())} 
          />
        </div>
        {open && page === SidebarPage.FileBrowser &&
        <div className={`${styles.SidebarBody} shadow-sm`}>
          <div className='w-full text-sm bg-gray-100'>
            search..
          </div>
          <SampleBrowser />
        </div>}
      </div>
    </SidebarBase>
  )
}

export default Sidebar
