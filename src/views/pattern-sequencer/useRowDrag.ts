import React from 'react'

// a row being dragged by its grip: which, and the gap it would go into (0 is
// above the first row, the number of rows below the last)
export interface RowDrag {
  rowId: number
  gap: number
}

// where a row dropped into `gap` goes, counted without it (as the engine
// takes it), or null when that's where it already is
export const moveIndex = (from: number, gap: number): number | null => {
  const to = gap > from ? gap - 1 : gap
  return to === from ? null : to
}

// a tab shows some of the rows. a row moved to `index` among those shown
// goes, among all the rows (both counted without it), just before the
// shown row now at `index`, or just after the last one shown (with no other
// shown, it stays where it is)
export const indexAmongAll = (allIds: number[], shownIds: number[], rowId: number, index: number): number => {
  const all = allIds.filter(id => id !== rowId)
  const shown = shownIds.filter(id => id !== rowId)
  if(!shown.length) return allIds.indexOf(rowId)
  if(index < shown.length) return all.indexOf(shown[index])
  return all.indexOf(shown[shown.length - 1]) + 1
}

// the gap between rows nearest `clientY`: past the middle of a row is below it
export const gapAt = (mids: number[], clientY: number): number =>
  mids.filter(mid => clientY > mid).length

// dragging rows by their grips, inside `ref`, whose rows are marked with
// data-row-id. `onMove` is given the row and the index to move it to. Escape,
// or the window losing focus, drops the drag
export const useRowDrag = (
  ref: React.RefObject<HTMLElement>,
  rowIds: number[],
  onMove: (rowId: number, index: number) => void,
) => {
  const [drag, setDrag] = React.useState<RowDrag | null>(null)
  const latest = React.useRef({ rowIds, onMove, drag })
  latest.current = { rowIds, onMove, drag }
  const dragging = drag !== null

  React.useEffect(() => {
    if(!dragging) return undefined

    const mids = () => Array.from(ref.current?.querySelectorAll<HTMLElement>('[data-row-id]') ?? [])
      .map(el => {
        const box = el.getBoundingClientRect()
        return box.top + box.height / 2
      })

    const move = (ev: MouseEvent) => {
      const gap = gapAt(mids(), ev.clientY)
      setDrag(d => (d && d.gap !== gap ? { ...d, gap } : d))
    }
    const drop = () => {
      const { drag: d, rowIds: ids, onMove: moved } = latest.current
      setDrag(null)
      if(!d) return
      const to = moveIndex(ids.indexOf(d.rowId), d.gap)
      if(to !== null) moved(d.rowId, to)
    }
    const cancel = () => setDrag(null)
    const escape = (ev: KeyboardEvent) => {
      if(ev.key === 'Escape') cancel()
    }

    const cursor = document.body.style.cursor
    document.body.style.cursor = 'grabbing'
    window.addEventListener('mousemove', move)
    window.addEventListener('mouseup', drop)
    window.addEventListener('keydown', escape)
    window.addEventListener('blur', cancel)
    return () => {
      document.body.style.cursor = cursor
      window.removeEventListener('mousemove', move)
      window.removeEventListener('mouseup', drop)
      window.removeEventListener('keydown', escape)
      window.removeEventListener('blur', cancel)
    }
  }, [dragging, ref])

  // a grip pressed: start dragging its row
  const onGripDown = React.useCallback((rowId: number, ev: React.MouseEvent) => {
    if(ev.button !== 0) return
    ev.preventDefault()
    const from = latest.current.rowIds.indexOf(rowId)
    if(from >= 0) setDrag({ rowId, gap: from })
  }, [])

  // the line showing where it would go: above or below which row (none
  // while it would stay where it is)
  let line: { rowId: number, edge: 'before' | 'after' } | null = null
  if(drag && moveIndex(rowIds.indexOf(drag.rowId), drag.gap) !== null) {
    line = drag.gap < rowIds.length
      ? { rowId: rowIds[drag.gap], edge: 'before' }
      : { rowId: rowIds[rowIds.length - 1], edge: 'after' }
  }

  return { draggingId: drag?.rowId ?? null, line, onGripDown }
}
