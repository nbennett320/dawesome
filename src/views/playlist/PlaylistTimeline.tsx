import React from 'react'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import {
  selectPlaylistPlaying,
} from '../../state/slices/playlistSlice'
import styles from './styles.module.scss'

const zeroPad = (num: number, places: number): string => (num.toString()).padStart(places, '0')

const PlaylistTimeline = () => {
  const [limit, setLimit] = React.useState(20)
  const [current, setCurrent] = React.useState(0)
  const playing = useAppSelector(selectPlaylistPlaying)
  const ref = React.useRef<any>(null)
  const range = [...Array(limit).keys()]

  // React.useEffect(() => {

  //   if(playing) {
      setInterval(() => {
        if(current + 1 < limit) setCurrent(current+1)
        else setCurrent(0)
      }, 500)
  //   } 
  // }, [playing])

  return (
    <div className={`${styles.PlaylistTimelineContainer}`}>
      <div className={`${styles.PlaylistTimeline} bg-slate-300`}>
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
      <div className={`${styles.PlayheadTrack}`}>
        <div style={{ position: 'fixed', left: `${100 * (current / (limit+1))}%`, borderLeftColor: 'red', height: '20px', borderWidth: '1px', borderStyle: 'solid', fontFamily: 'monospace', color: 'rgba(0,0,0,0)' }}>ee</div> 
      </div>
    </div>
  )
}

export default PlaylistTimeline
