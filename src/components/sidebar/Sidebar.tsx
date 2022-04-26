import React from 'react'
import {
  toggleSidebar,
  selectSidebar,
} from '../../state/slices/windowSlice'
import { 
  getPlaylistSamples,
  selectPlaylistSamples,
} from '../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import Draggable from '../drag-and-drop/Draggable'
import { DragEffectEnum } from '../../types/dragAndDrop'
import SidebarBase from './SidebarBase'
import SidebarHeader from './SidebarHeader'
import styles from './styles.module.scss'

interface Props {
}

const Sidebar = (props: React.PropsWithChildren<Props>) => {
  const open = useAppSelector(selectSidebar)
  const samples = useAppSelector(selectPlaylistSamples)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    dispatch(getPlaylistSamples())
  }, [])

  return (
    <SidebarBase open={open}>
      <div className={`${styles.Sidebar} bg-gray-300`}>
        <div className={styles.SidebarHeaderContainer}>
          <SidebarHeader onClose={() => dispatch(toggleSidebar())} />
        </div>
        <div className={styles.SidebarBody}>
          {samples.map((e) => (
            <Draggable 
              item={e}
              type={DragEffectEnum.Link}
            >
              <span className='text-xs text-ellipsis whitespace-nowrap overflow-hidden w-full'>
                {e}
              </span>
            </Draggable>
          ))}
        </div>
      </div>
    </SidebarBase>
  )
}

export default Sidebar
