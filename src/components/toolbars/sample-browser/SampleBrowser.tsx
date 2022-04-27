import React from 'react'
import { 
  getPlaylistSamples,
  selectPlaylistSamples,
} from '../../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../../hooks/redux'
import SampleItem from './SampleItem'
import styles from './styles.module.scss'

interface Props {
}

const SampleBrowser = (props: React.PropsWithChildren<Props>) => {
  const samples = useAppSelector(selectPlaylistSamples)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    dispatch(getPlaylistSamples())
  }, [])

  return (
    <div className={`${styles.SampleBrowser}`}>
      <div className={styles.SampleBrowserItemContainer}>
        {samples.map((e, i) => (
          <SampleItem 
            name={e}
            key={`${e}-${i as unknown as string}`}
          />
        ))}
      </div>
    </div>
  )
}

export default SampleBrowser
