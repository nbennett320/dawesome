import React from 'react'
import { DragItem, onSampleDrag } from '../../../state/sampleDrag'

// marks where something dropped goes: a folder's row (its key), a row inside
// a folder (that folder's key), or the list itself (the top, '')
export const DROP_FOLDER = 'data-drop-folder'

// how long something's held over a closed folder before it opens
const OPEN_AFTER_MS = 700

interface Options {
  // whether `item` can go into the folder `key`
  canDrop: (item: DragItem, key: string) => boolean
  onDrop: (item: DragItem, key: string) => void
  // held over a folder for a moment: open it, to drop further in
  onHold?: (key: string) => void
}

// dropping things into the folders of a browser in the sidebar. returns the
// folder that what's being dragged would go into, to show it
export const useFolderDrop = (ref: React.RefObject<HTMLElement>, options: Options): string | null => {
  const [target, setTarget] = React.useState<string | null>(null)
  const latest = React.useRef(options)
  latest.current = options

  React.useEffect(() => {
    let held: { key: string, timer: ReturnType<typeof setTimeout> } | null = null
    const release = () => {
      if(held) clearTimeout(held.timer)
      held = null
    }

    const stop = onSampleDrag(({ type, item, clientX, clientY }) => {
      const box = ref.current
      const under = document.elementFromPoint(clientX, clientY)
      const zone = box && under && box.contains(under)
        ? under.closest<HTMLElement>(`[${DROP_FOLDER}]`)
        : null
      const key = zone?.getAttribute(DROP_FOLDER) ?? null
      const ok = key !== null && latest.current.canDrop(item, key)

      if(type === 'move') {
        setTarget(ok ? key : null)
        if(held?.key !== key) {
          release()
          if(ok && key) {
            held = { key, timer: setTimeout(() => latest.current.onHold?.(key), OPEN_AFTER_MS) }
          }
        }
        return
      }

      release()
      setTarget(null)
      if(type === 'drop' && ok) latest.current.onDrop(item, key as string)
    })

    return () => {
      release()
      stop()
    }
  }, [ref])

  return target
}
