import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from 'state/store'
import { useAppDispatch } from '../../hooks/redux'
import { draggedInstrument, EffectDragItem, isEffectItem, onSampleDrag } from '../../state/sampleDrag'
import { addEffect, highlightSlot, moveEffect } from '../../state/slices/effectsSlice'
import {
  ChannelId, findSlot, fitGap, isInstrument, reorderIndex, slotsOf,
} from '../../types/effects'

// marks a row of the stack, so a drop can tell which gap it's in
export const RACK_ROW = 'data-rack-row'

// the gap between rows the mouse is in: 0 is above the first row, and
// mids.length below the last. `mids` are the rows' middles, top to bottom
export const gapAt = (mids: number[], clientY: number): number =>
  mids.filter(mid => mid < clientY).length

// the gap a drop would go in, where the mouse is in `gap`: an effect can't
// go above a track's instrument, and an instrument only goes first
export const rackGap = (
  racks: RootState['effects']['racks'],
  channel: ChannelId,
  item: EffectDragItem,
  gap: number,
): number => fitGap(slotsOf(racks, channel), gap, draggedInstrument(racks, item))

// what dropping `item` in `gap` of a channel's chain does: insert an effect
// there, or move a slot there. an instrument is added as the track's (the
// engine puts it first: `index` null), or moved to the head of another
// track's chain. null when it'd change nothing or be refused: an instrument
// on the master, moved within its own chain, or onto a track that has one
export const rackDrop = (
  racks: RootState['effects']['racks'],
  channel: ChannelId,
  item: EffectDragItem,
  mouseGap: number,
): { kind: 'add', index: number | null } | { kind: 'move', index: number } | null => {
  const instrument = draggedInstrument(racks, item)
  if(instrument && channel === 'master') return null

  const slots = slotsOf(racks, channel)
  const gap = rackGap(racks, channel, item, mouseGap)
  if(item.kind === 'effect') return { kind: 'add', index: instrument ? null : gap }

  const from = findSlot(racks, item.slotId)
  if(!from) return null

  if(instrument) {
    if(from.channel === channel || slots.some(isInstrument)) return null
    return { kind: 'move', index: 0 }
  }

  if(from.channel === channel) {
    const index = reorderIndex(slots, item.slotId, gap)
    return index === null ? null : { kind: 'move', index }
  }
  return { kind: 'move', index: gap }
}

// effects dropped into the stack: one from the sidebar goes where it's
// dropped (an instrument at the top, as the track's), and a row moves
// there. returns the gap it'd go in, to show it
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
      setTarget(drop ? rackGap(racks, channel, item, gap) : null)
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
