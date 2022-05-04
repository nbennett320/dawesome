import React from 'react'
import { useDrag } from 'react-dnd'

interface Props {
  value: string
}

const PlaylistItem = (props: Props) => {
  

  return (
    <div>
      {props.value}
    </div>
  )
}

export default PlaylistItem
