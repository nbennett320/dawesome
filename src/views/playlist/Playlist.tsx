import React from 'react'
import Droppable from '../../components/drag-and-drop/Droppable'
import { DropEffectEnum } from '../../types/dragAndDrop'
import PlaylistTimeline from './PlaylistTimeline'
import styles from './styles.module.scss'

const Playlist = () => {
  const [items, setItems] = React.useState<string[]>([])
  console.log(items)

  return (
    <div className={`${styles.Playlist} h-full`}>
      <PlaylistTimeline />
      <div className='h-full'>
        <Droppable 
          onDrop={(item) => {console.log(item);setItems([...items, item])}}
          type={DropEffectEnum.Link}
        >
          {items.map(e => (
            <span key={e}>{e}</span>
          ))}
        </Droppable>
      </div>
    </div>
  )
}

export default Playlist
