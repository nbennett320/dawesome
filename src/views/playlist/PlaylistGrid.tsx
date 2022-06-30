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
        style={{ width: `${100 * ratio}%` }}
        className={`${styles.PlaylistGrid} bg-slate-300`}
      >
        {range.map(e => (
          <div 
            key={`grid-segment-${e}`}
            id={`playlist-grid-segment-${e}`}
            style={{ width: `${1 / limit}%` }}
            className={`${styles.PlaylistGridSegment} border-l-slate-400`}
          />
        ))}
        
      </div>
    </div>
  )
}

export default PlaylistGrid
