import React from 'react'
import { DropEffect } from '../../types/dragAndDrop'
import styles from './styles.module.scss'

interface Props {
  onDrop: (data: string, ev: React.DragEvent<HTMLDivElement>) => void
  type: DropEffect
}

const Droppable = (props: React.PropsWithChildren<Props>) => {
  const [over, setOver] = React.useState(false)

  const handleDragOver = (ev: React.DragEvent<HTMLDivElement>) => {
    ev.preventDefault()
    ev.dataTransfer.dropEffect = props.type
  }

  const handleDrop = (ev: React.DragEvent<HTMLDivElement>) => {
    const droppedItem = ev.dataTransfer.getData("drag-item")
    if(droppedItem) {
      props.onDrop(droppedItem, ev)
    }
    setOver(false)
  }
  
  const handleDragEnter = (ev: React.DragEvent<HTMLDivElement>) => {
    ev.dataTransfer.dropEffect = props.type
    setOver(true)
  }
  
  const handleDragLeave = (ev: React.DragEvent<HTMLDivElement>) => {
    setOver(false)
  }
  
  return (
    <div
      className={`${over ? styles.DroppableIsOver : styles.Droppable}`}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
    >
      {props.children}
    </div>
  )
}

export default Droppable
