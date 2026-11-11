import React from 'react'
import {
  pressSidebarPage,
  selectSidebar,
  selectSidebarPage,
} from '../../state/slices/windowSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { SidebarPage } from '../../types/ui'
import SidebarBase from './SidebarBase'
import SidebarList from './SidebarList'
import SampleBrowser from '../toolbars/sample-browser/SampleBrowser'
import PatternBrowser from '../toolbars/pattern-browser/PatternBrowser'
import RecordingsBrowser from '../toolbars/recordings-browser/RecordingsBrowser'
import ClipsBrowser from '../toolbars/clips-browser/ClipsBrowser'
import styles from './styles.module.scss'

const pages: Record<SidebarPage, () => React.ReactElement> = {
  [SidebarPage.Samples]: () => <SampleBrowser />,
  [SidebarPage.Patterns]: () => <PatternBrowser />,
  [SidebarPage.Recordings]: () => <RecordingsBrowser />,
  [SidebarPage.Clips]: () => <ClipsBrowser />,
}

const Sidebar = () => {
  const open = useAppSelector(selectSidebar)
  const page = useAppSelector(selectSidebarPage)
  const dispatch = useAppDispatch()
  const Page = pages[page]

  return (
    <SidebarBase open={open}>
      <div className={`${styles.Sidebar} flex flex-row`}>
        <div className={styles.SidebarListContainer}>
          <SidebarList
            open={open}
            page={page}
            onPress={next => dispatch(pressSidebarPage(next))}
          />
        </div>
        {open && (
          <div className={`${styles.SidebarBody} w-60 bg-surface border-r border-line`}>
            <Page />
          </div>
        )}
      </div>
    </SidebarBase>
  )
}

export default Sidebar
