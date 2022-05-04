import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrag } from 'react-dnd'
import { PlaylistTypes } from '../../../types/playlist'

interface Props {
  name: string
}

const SampleItem = (props: Props) => {
  const [{ isDragging }, drag] = useDrag(() => ({
    type: PlaylistTypes.PlaylistItem,
    item: { name: props.name },
    end: (item, monitor) => {
      const dropResult = monitor.getDropResult<Props>()
      if (item && dropResult) {
        console.log("dropped, from sample item")
      }
    },
    collect: (monitor) => ({
      isDragging: monitor.isDragging(),
      handlerId: monitor.getHandlerId(),
    }),
  }))

  if(isDragging) console.log("dragging")

  const previewSample = () => {
    console.log("playing")
    invoke('preview_sample', {
      path: props.name
    })
  }

  return (
    <span
      ref={drag}
      className='text-xs text-ellipsis whitespace-nowrap overflow-hidden w-full'
      onMouseDown={previewSample}
      role='button'
    >
      {props.name}
    </span>
  )
}

export default SampleItem
