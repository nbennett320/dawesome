import React from 'react'
import { cssVar } from '../theme/apply'
import { dragItemLabel, onSampleDrag } from '../state/sampleDrag'

// an element marked with this draws what's dragged over it itself (the
// playlist canvas draws the whole clip), so the label hides over it
export const OWN_DRAG_PREVIEW = 'data-own-drag-preview'

interface Ghost {
  name: string
  x: number
  y: number
}

// matches the label height of clips in the playlist canvas
const LABEL_HEIGHT = 20

// a clip label pinned to the mouse while a sample is dragged from the sidebar
// (or a pattern from the Sequencer).
// shown everywhere except over the playlist, where the canvas takes over and
// draws the full clip and where it will land
//
// mounted once, at the top of the app, and kept in its own component so
// following the mouse doesn't re-render anything else
const DragGhost = () => {
  const [ghost, setGhost] = React.useState<Ghost | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    const overPlaylist = !!document.elementFromPoint(clientX, clientY)
      ?.closest(`[${OWN_DRAG_PREVIEW}]`)

    if(type !== 'move' || overPlaylist) {
      setGhost(null)
      return
    }

    setGhost({
      name: dragItemLabel(item),
      x: clientX,
      y: clientY,
    })
  }), [])

  if(!ghost) return null

  return (
    <div
      className="text-xs text-ink whitespace-nowrap overflow-hidden text-ellipsis rounded shadow-sm"
      style={{
        position: 'fixed',
        // held by the middle of its label, like the clip in the canvas
        left: ghost.x,
        top: ghost.y - LABEL_HEIGHT / 2,
        height: LABEL_HEIGHT,
        maxWidth: '12rem',
        padding: '0 6px',
        lineHeight: `${LABEL_HEIGHT}px`,
        // like a clip's label in the playlist (the playlist's clipBorder and clipLabel)
        border: `1px solid ${cssVar('lineStrong')}`,
        background: cssVar('panel', 0.95),
        pointerEvents: 'none',
        zIndex: 1000,
      }}
    >
      {ghost.name}
    </div>
  )
}

export default DragGhost
