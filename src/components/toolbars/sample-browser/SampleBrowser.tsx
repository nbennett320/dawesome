import React from 'react'
import {
  toggleSidebar,
  selectSidebar,
} from '../../../state/slices/windowSlice'
import { 
  getPlaylistSamples,
  selectPlaylistSamples,
} from '../../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../../hooks/redux'
import SampleItem from './SampleItem'
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
    <div className={`${styles.Sidebar}`}>
      <div className={styles.SidebarBody}>
        {samples.map((e, i) => (
          <SampleItem 
            name={e}
            key={`${e}-${i}`}
          />
        ))}
      </div>
    </div>
  )
}

export default Sidebar
