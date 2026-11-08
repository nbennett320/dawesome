import React from 'react'
import { onSampleDrag } from '../../state/sampleDrag'

interface Props {
  // the playlist, where the canvas draws the dragged clip itself
  playlistRef: React.RefObject<HTMLElement>
}

interface Ghost {
  name: string
  x: number
  y: number
}

// matches the label height of clips in the playlist canvas
const LABEL_HEIGHT = 20

// a clip label pinned to the mouse while a sample is dragged from the sidebar.
// shown everywhere except over the playlist, where the canvas takes over and
// draws the full clip and where it will land
//
// kept in its own component so following the mouse doesn't re-render the
// playlist canvas
const SampleDragGhost = ({ playlistRef }: Props) => {
  const [ghost, setGhost] = React.useState<Ghost | null>(null)

  React.useEffect(() => onSampleDrag(({ type, path, clientX, clientY }) => {
    const overPlaylist = !!playlistRef.current?.contains(
      document.elementFromPoint(clientX, clientY))

    if(type !== 'move' || overPlaylist) {
      setGhost(null)
      return
    }

    setGhost({
      name: path.split(/[\\/]/).pop() ?? path,
      x: clientX,
      y: clientY,
    })
  }), [])

  if(!ghost) return null

  return (
    <div
      className="text-xs text-gray-800 whitespace-nowrap overflow-hidden text-ellipsis rounded"
      style={{
        position: 'fixed',
        // held by the middle of its label, like the clip in the canvas
        left: ghost.x,
        top: ghost.y - LABEL_HEIGHT / 2,
        height: LABEL_HEIGHT,
        maxWidth: '12rem',
        padding: '0 6px',
        lineHeight: `${LABEL_HEIGHT}px`,
        border: '1px solid rgb(125, 211, 252)',
        background: 'rgba(224, 242, 254, 0.9)',
        pointerEvents: 'none',
        zIndex: 1000,
      }}
    >
      {ghost.name}
    </div>
  )
}

export default SampleDragGhost
