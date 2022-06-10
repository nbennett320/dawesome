import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrag } from 'react-dnd'
import PlaylistItemWaveform from './PlaylistItemWaveform'
import { PlaylistItemWaveformData } from '../../types/playlist'
import styles from './styles.module.scss'

interface Props {
  onRightClick: (id: number) => void,
  id: number,
  value: string,
  pixelOffset: number,
}

const PlaylistItem = (props: Props) => {
  const [data, setData] = React.useState<PlaylistItemWaveformData>({
    pathd: '',
    viewBox: '',
  })

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
      onContextMenu={handleRightClick}
      className={`${styles.PlaylistItem} border-2`}
      style={{
        left: `${props.pixelOffset}px`
      }}
    >
      <span className='text-xs bg-slate-200'>{props.value}</span>
      <PlaylistItemWaveform 
        path={data.pathd}
        viewBox={data.viewBox}
      />
    </div>
  )
}

export default PlaylistItem
