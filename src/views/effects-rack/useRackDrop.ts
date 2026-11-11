import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from 'state/store'
import { useAppDispatch } from '../../hooks/redux'
import { EffectDragItem, isEffectItem, onSampleDrag } from '../../state/sampleDrag'
import { addEffect, highlightSlot, moveEffect } from '../../state/slices/effectsSlice'
import {
  ChannelId, findSlot, reorderIndex, slotsOf,
} from '../../types/effects'

// marks a row of the stack, so a drop can tell which gap it's in
export const RACK_ROW = 'data-rack-row'

// the gap between rows the mouse is in: 0 is above the first row, and
// mids.length below the last. `mids` are the rows' middles, top to bottom
export const gapAt = (mids: number[], clientY: number): number =>
  mids.filter(mid => mid < clientY).length

// what dropping `item` in `gap` of a channel's chain does: insert an effect
// there, or move a slot there. null when it'd change nothing
export const rackDrop = (
  racks: RootState['effects']['racks'],
  channel: ChannelId,
  item: EffectDragItem,
  gap: number,
): { kind: 'add', index: number } | { kind: 'move', index: number } | null => {
  const slots = slotsOf(racks, channel)
  if(item.kind === 'effect') return { kind: 'add', index: gap }

  const from = findSlot(racks, item.slotId)
  if(!from) return null

  if(from.channel === channel) {
    const index = reorderIndex(slots, item.slotId, gap)
    return index === null ? null : { kind: 'move', index }
  }
  return { kind: 'move', index: gap }
}

// effects dropped into the stack: one from the sidebar goes where it's
// dropped, and a row moves there. returns the gap it'd go in, to show it
export const useRackDrop = (ref: React.RefObject<HTMLElement>, channel: ChannelId): number | null => {
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const [target, setTarget] = React.useState<number | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    if(!isEffectItem(item)) return

    const box = ref.current
    const under = document.elementFromPoint(clientX, clientY)
    const over = !!box && !!under && box.contains(under)
    const mids = over
      ? Array.from(box.querySelectorAll<HTMLElement>(`[${RACK_ROW}]`)).map(row => {
        const { top, height } = row.getBoundingClientRect()
        return top + height / 2
      })
      : []
    const { racks } = store.getState().effects
    const gap = gapAt(mids, clientY)
    const drop = over ? rackDrop(racks, channel, item, gap) : null

    if(type === 'move') {
      setTarget(drop ? gap : null)
      return
    }

    setTarget(null)
    if(type !== 'drop' || !drop) return

    if(drop.kind === 'add' && item.kind === 'effect') {
      dispatch(addEffect(channel, item.pluginId, drop.index))
        .then(slotId => { if(slotId !== null) dispatch(highlightSlot(slotId)) })
        .catch(() => {})
    } else if(drop.kind === 'move' && item.kind === 'slot') {
      dispatch(moveEffect(item.slotId, channel, drop.index)).catch(() => {})
    }
  }), [dispatch, ref, store, channel])

  return target
}
