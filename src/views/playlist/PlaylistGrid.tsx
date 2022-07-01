import React from 'react'
import { invoke } from '@tauri-apps/api'
import styles from './styles.module.scss'

const PlaylistGrid = () => {
  const [limit, setLimit] = React.useState<number>(16)
  const [ratio, setRatio] = React.useState<number>(1)
  const range = [...Array(limit).keys()].map(e => e + 1)

  React.useEffect(() => {
    const getPlaylistTimeline = async () => {
      const [maxPlaylistBeats, maxBeatsDisplayed, displayRatio] = 
        await invoke<[number, number, number]>('get_playlist_timeline', {})
      setLimit(maxPlaylistBeats)
      setRatio(displayRatio)
    }

    getPlaylistTimeline()
  }, [])

  return (
    <div className={`${styles.PlaylistGridContainer}`}>
      <div 
        style={{ 
          width: `100%`,
          height: `100vh`,
          backgroundImage: `
            repeating-linear-gradient(
              rgb(203 213 225 / var(--tw-bg-opacity)) 0 1px, 
              transparent 1px 100%
            ),
            repeating-linear-gradient(
              90deg, 
              rgb(203 213 225 / var(--tw-bg-opacity)) 0 1px,
              transparent 1px 100%
            )`,
          backgroundSize: `${101}px ${101}px`,
        }}
        className={`${styles.PlaylistGrid}`}
      />
    </div>
  )
}

export default PlaylistGrid
