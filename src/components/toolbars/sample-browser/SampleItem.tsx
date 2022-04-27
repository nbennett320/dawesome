import React from 'react'
import { invoke } from '@tauri-apps/api'
import Draggable from '../../drag-and-drop/Draggable'
import { DragEffectEnum } from '../../../types/dragAndDrop'

interface Props {
  name: string
}

const SampleItem = (props: Props) => {
  const previewSample = () => {
    console.log("plaing")
    invoke('preview_sample', {
      path: props.name
    })
  }

  return (
    <Draggable 
      item={props.name}
      type={DragEffectEnum.Link}
    >
      <span
        className='text-xs text-ellipsis whitespace-nowrap overflow-hidden w-full'
        onMouseDown={previewSample}
      >
        {props.name}
      </span>
    </Draggable>
  )
}

export default SampleItem
