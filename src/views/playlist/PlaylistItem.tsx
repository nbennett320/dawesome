import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrag } from 'react-dnd'
import PlaylistItemWaveform from './PlaylistItemWaveform'
import { PlaylistTypes, PlaylistItemWaveformData } from '../../types/playlist'
import styles from './styles.module.scss'

interface Props {
  onRightClick: (id: number) => void,
  id: number,
  value: string,
  pixelOffset: number,
}

const PlaylistItem = (props: Props) => {
  const [{ isDragging }, drag] = useDrag(() => ({
    type: PlaylistTypes.PlaylistTrackItem,
    item: { 
      name: props.value,
      id: props.id,
    },
    end: (item, monitor) => {
      const dropResult = monitor.getDropResult<Props>()
      if (item && dropResult) {
        console.log("dropped, after dragging node from playlist")
      }
    },
    collect: (monitor) => ({
      isDragging: monitor.isDragging(),
      handlerId: monitor.getHandlerId(),
    }),
  }))

  if(isDragging) console.log("dragging node from playlist")

  const [data, setData] = React.useState<PlaylistItemWaveformData>({
    pathd: '',
    viewBox: '',
  })
  const [width, setWidth] = React.useState<number>(0)

  React.useEffect(() => {
    const fetchData = async () => {
      const [pathd, viewBox] = await invoke<string[]>('get_node_data', {
        id: props.id
      })

      setData({
        pathd,
        viewBox
      })
    }
    
    fetchData()
  }, [])

  const handleRightClick = (e: React.MouseEvent<HTMLDivElement, MouseEvent>) => {
    e.preventDefault()
    props.onRightClick(props.id)
  }

  return (
    <div 
      ref={drag}
      onContextMenu={handleRightClick}
      className={`${styles.PlaylistItem} border-2`}
      style={{
        left: `${props.pixelOffset}px`,
        width: `${width}px`,
      }}
    >
      <span 
        className={`${styles.PlaylistItemTitle} text-xs bg-slate-200`}
      >
        {props.value}
      </span>

      <PlaylistItemWaveform 
        path={data.pathd}
        viewBox={data.viewBox}
        setItemWidth={setWidth}
      />
    </div>
  )
}

export default PlaylistItem
