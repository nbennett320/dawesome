import React from 'react'
import { DragEffect } from '../../types/dragAndDrop'
import styles from './styles.module.scss'

interface Props {
  item: string
  type: DragEffect
  image?: string
}

const Draggable = (props: React.PropsWithChildren<Props>) => {
  const [dragging, setDragging] = React.useState(false)
  const draggedImage = React.useRef<HTMLImageElement | null>(null)

  React.useEffect(() => {
    draggedImage.current = null

    if(props?.image) {
      draggedImage.current = new Image()
      draggedImage.current.src = props.image
    }

  }, [props.image])
  
  const handleDragStart = (ev: React.DragEvent<HTMLDivElement>) => {
    setDragging(true)
    
    const data = {
      props
    }
    console.log(data)
    ev.dataTransfer.setData("drag-item", props.item)
    ev.dataTransfer.effectAllowed = props.type
    if(draggedImage.current) {
      ev.dataTransfer.setDragImage(draggedImage.current, 0, 0)
    }
  }
  
  const handleDragEnd = (ev: React.DragEvent<HTMLDivElement>) => {
    setDragging(false)
  }

  return (
    <div
      className={`${dragging ? styles.DraggableDragging : ''} overflow-hidden text-ellipsis`}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      draggable
    >
      {props.children}
    </div>
  )
}

export default Draggable
