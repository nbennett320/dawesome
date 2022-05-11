import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrag } from 'react-dnd'
import PlaylistItemWaveform from './PlaylistItemWaveform'

interface Props {
  id: number
  value: string
}

const PlaylistItem = (props: Props) => {
  const [data, setData] = React.useState('')

  React.useEffect(() => {
    const fetchData = async () => {
      const newData = await invoke<string>('get_node_data', {
        id: props.id
      })

      setData(newData)
    }
    
    fetchData()
  }, [data])

  return (
    <div>
      <span className='text-xs'>{props.value}</span>
      <PlaylistItemWaveform 
        path={data}
      />
    </div>
  )
}

export default PlaylistItem
