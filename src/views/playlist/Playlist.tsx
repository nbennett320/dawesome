import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrop } from 'react-dnd'
import { 
  addToPlaylist,
  selectPlaylistItems
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import PlaylistTimeline from './PlaylistTimeline'
import PlaylistItem from './PlaylistItem'
import { PlaylistTypes } from '../../types/playlist'
import styles from './styles.module.scss'

const Playlist = () => {
  const dispatch = useAppDispatch()
  const playlistBoxRef = React.useRef<HTMLDivElement>(null)
  const [{ canDrop, isOver }, dropRef] = useDrop(() => ({
    accept: PlaylistTypes.PlaylistItem,
    drop: async (item: Record<string, any>, monitor) => {
      console.log(item, monitor.getClientOffset())
      const offset = await invoke<number>('get_playlist_sample_offset', {
        dropX: monitor.getClientOffset()?.x,
        dropY: monitor.getClientOffset()?.y,
        minBoundX: playlistBoxRef.current?.getBoundingClientRect().left,
        minBoundY: playlistBoxRef.current?.getBoundingClientRect().top,
        maxBoundX: playlistBoxRef.current?.getBoundingClientRect().right,
        maxBoundY: playlistBoxRef.current?.getBoundingClientRect().bottom,
      })

      dispatch(addToPlaylist(item.name as string, offset))
    },
    collect: (monitor) => ({
      isOver: monitor.isOver(),
      canDrop: monitor.canDrop(),
    }),
  }))

  const items = useAppSelector(selectPlaylistItems)
  const isActive = canDrop && isOver

  return (
    <div 
      ref={dropRef}
      data-testid='playlist'
      className={`${styles.Playlist} h-full`}
    >
      <PlaylistTimeline />
      <div
        ref={playlistBoxRef} 
        className={`${styles.PlaylistBodyContainer} h-full`}
      >
        {items.map((e, i) => (
          <PlaylistItem 
            key={`${e}-${i as unknown as string}`}
            id={e.id}
            value={e.path}
          />
        ))}
      </div>
    </div>
  )
}

export default Playlist
