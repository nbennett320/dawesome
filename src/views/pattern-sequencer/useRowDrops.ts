import React from 'react'
import { useStore } from 'react-redux'
import { useAppDispatch } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import {
  addSampleRow, addSynthRow, replaceRowSample, replaceRowSynth,
} from '../../state/slices/sequencerSlice'
import { DragItem, dragItemKey, onSampleDrag } from '../../state/sampleDrag'
import { Row } from '../../types/sequencer'
import { fileName } from '../../util/helpers'

// what becomes a row when it's dropped on the Pattern Sequencer
export type RowItem = Extract<DragItem, { kind: 'sample' | 'instrument' }>

const isRowItem = (item: DragItem): item is RowItem => item.kind === 'sample' || item.kind === 'instrument'

// the kind of row a dragged thing makes
const kindOf = (item: RowItem): Row['instrument']['kind'] => (item.kind === 'sample' ? 'sample' : 'synth')

// what a dragged thing is called: a sample's file, an instrument's name
export const itemName = (item: RowItem): string => (item.kind === 'sample' ? fileName(item.path) : item.label)

// a sample or an instrument dragged over the tab: what it is, and the row it
// would replace (one of its kind under the mouse), if any
export interface RowDrop {
  item: RowItem
  rowId: number | null
}

// the row at a point in the window, inside `tab`, if there's one
const rowAt = (tab: HTMLElement | null, clientX: number, clientY: number): number | null => {
  const el = document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[data-row-id]')
  return el && tab?.contains(el) ? Number(el.dataset.rowId) : null
}

// samples and instruments dragged from the sidebar onto `tab`. let go on a
// row of their own kind, they replace its sample or plugin (its notes and
// settings stay); anywhere else on the tab, they're added as a row at the
// bottom. returns what's being dragged over the tab now, re-rendering only
// when that, or the row it would replace, changes
export const useRowDrops = (tab: React.RefObject<HTMLElement>): RowDrop | null => {
  const store = useStore<RootState>()
  const dispatch = useAppDispatch()
  const [drop, setDrop] = React.useState<RowDrop | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    if(type === 'cancel' || !isRowItem(item) || !tab.current?.contains(document.elementFromPoint(clientX, clientY))) {
      setDrop(null)
      return
    }
    const under = store.getState().sequencer.rows.find(row => row.id === rowAt(tab.current, clientX, clientY))
    const rowId = under?.instrument.kind === kindOf(item) ? under.id : null

    if(type === 'move') {
      const same = (now: RowDrop | null) => now && dragItemKey(now.item) === dragItemKey(item) && now.rowId === rowId
      setDrop(now => (same(now) ? now : { item, rowId }))
      return
    }
    setDrop(null)
    if(item.kind === 'instrument') {
      if(rowId === null) dispatch(addSynthRow(item.pluginId))
      else dispatch(replaceRowSynth(rowId, item.pluginId))
    } else if(rowId === null) {
      dispatch(addSampleRow(item.path))
    } else {
      dispatch(replaceRowSample(rowId, item.path))
    }
  }), [tab, store, dispatch])

  return drop
}
