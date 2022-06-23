import React from 'react'
import { invoke } from '@tauri-apps/api'
import styles from './styles.module.scss'

const zeroPad = (num: number, places: number): string => (num.toString()).padStart(places, '0')

const PlaylistTimeline = () => {
  const [limit, setLimit] = React.useState<number>(16)
  const [ratio, setRatio] = React.useState<number>(1)
  const range = [...Array(limit - 1).keys()].map(e => e + 1)

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
    <div className={`${styles.PlaylistTimelineContainer}`}>
      <div 
        style={{ width: `${100 * ratio}%` }}
        className={`${styles.PlaylistTimeline} bg-slate-300`}
      >
        {range.map(e => (
          <div 
            key={`segment-${e}`}
            id={`playlist-segment-${e}`}
            className={`${styles.PlaylistTimelineSegment} text-gray-600 text-xs font-mono`}
          >
            {e < 10 ? zeroPad(e, 1) : e}
          </div>
        ))}
        
      </div>
      {/* <div className={`${styles.PlayheadTrack}`}>
        <div style={{ position: 'fixed', left: `${100 * (current / (limit+1))}%`, borderLeftColor: 'red', height: '20px', borderWidth: '1px', borderStyle: 'solid', fontFamily: 'monospace', color: 'rgba(0,0,0,0)' }}>ee</div> 
      </div> */}
    </div>
  )
}

export default PlaylistTimeline
