import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from 'state/store'
import { useAppDispatch } from '../../hooks/redux'
import { draggedInstrument, EffectDragItem, isEffectItem, onSampleDrag } from '../../state/sampleDrag'
import { dropEffectOnChannel, moveEffect } from '../../state/slices/effectsSlice'
import {
  appendIndex, ChannelId, findSlot, isInstrument, parseChannelKey, Racks, slotsOf,
} from '../../types/effects'

// marks a mixer strip as a place effects can be dropped: its channel's key
// (see channelKey)
export const DROP_CHANNEL = 'data-drop-channel'

// the channel under the mouse, if it's a strip inside `box`
export const channelAt = (box: HTMLElement | null, clientX: number, clientY: number): ChannelId | null => {
  const under = document.elementFromPoint(clientX, clientY)
  if(!box || !under || !box.contains(under)) return null
  return parseChannelKey(under.closest(`[${DROP_CHANNEL}]`)?.getAttribute(DROP_CHANNEL))
}

// what dropping `item` on a channel's strip does: add a plugin (at the end
// of the chain, or as a track's instrument), or move a slot there. null when
// it'd be refused or change nothing: an instrument on the master, a track's
// instrument onto its own strip or onto a track that has one
export const channelDrop = (
  racks: Racks,
  channel: ChannelId,
  item: EffectDragItem,
): { kind: 'add' } | { kind: 'move', index: number } | null => {
  const instrument = draggedInstrument(racks, item)
  if(instrument && channel === 'master') return null
  if(item.kind === 'effect') return { kind: 'add' }

  const slots = slotsOf(racks, channel)
  if(instrument) {
    if(findSlot(racks, item.slotId)?.channel === channel || slots.some(isInstrument)) return null
    return { kind: 'move', index: 0 }
  }

  // an effect, to the end of this channel's chain, settings kept
  const index = appendIndex(slots, item.slotId)
  return index === null ? null : { kind: 'move', index }
}

// effects dropped on the mixer's strips: one from the sidebar goes at the
// end of that channel's chain (an instrument becomes the track's), and a
// row of the Effects Rack moves there. returns the channel what's dragged
// would go on, to show it
export const useChannelDrop = (ref: React.RefObject<HTMLElement>): ChannelId | null => {
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const [target, setTarget] = React.useState<ChannelId | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    if(!isEffectItem(item)) return

    const channel = channelAt(ref.current, clientX, clientY)
    const drop = channel === null ? null : channelDrop(store.getState().effects.racks, channel, item)
    if(type === 'move') {
      setTarget(drop ? channel : null)
      return
    }

    setTarget(null)
    if(type !== 'drop' || channel === null || !drop) return

    if(item.kind === 'effect') {
      dispatch(dropEffectOnChannel(channel, item.pluginId)).catch(() => {})
    } else if(drop.kind === 'move') {
      dispatch(moveEffect(item.slotId, channel, drop.index)).catch(() => {})
    }
  }), [dispatch, ref, store])

  return target
}
