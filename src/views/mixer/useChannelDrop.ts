import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from 'state/store'
import { useAppDispatch } from '../../hooks/redux'
import { DragItem, EffectDragItem, isEffectItem, onSampleDrag } from '../../state/sampleDrag'
import { dropEffectOnChannel, moveEffect } from '../../state/slices/effectsSlice'
import { addSynthOnTrack } from '../../state/slices/sequencerSlice'
import {
  appendIndex, ChannelId, parseChannelKey, Racks, slotsOf,
} from '../../types/effects'

// marks a mixer strip as a place effects can be dropped: its channel's key
// (see channelKey)
export const DROP_CHANNEL = 'data-drop-channel'

// what a strip takes: effects and rack rows, and instruments
export type ChannelDragItem = EffectDragItem | Extract<DragItem, { kind: 'instrument' }>

const isChannelItem = (item: DragItem): item is ChannelDragItem =>
  isEffectItem(item) || item.kind === 'instrument'

// the channel under the mouse, if it's a strip inside `box`
export const channelAt = (box: HTMLElement | null, clientX: number, clientY: number): ChannelId | null => {
  const under = document.elementFromPoint(clientX, clientY)
  if(!box || !under || !box.contains(under)) return null
  return parseChannelKey(under.closest(`[${DROP_CHANNEL}]`)?.getAttribute(DROP_CHANNEL))
}

// what dropping `item` on a channel's strip does: add an effect at the end
// of the chain, move a slot there, or (an instrument, on a track) add a
// synth row, with a pattern of it on that track. null when it'd be refused or
// change nothing: an instrument on the master, a slot already last there
export const channelDrop = (
  racks: Racks,
  channel: ChannelId,
  item: ChannelDragItem,
): { kind: 'add' } | { kind: 'synth' } | { kind: 'move', index: number } | null => {
  if(item.kind === 'instrument') return channel === 'master' ? null : { kind: 'synth' }
  if(item.kind === 'effect') return { kind: 'add' }

  // an effect, to the end of this channel's chain, settings kept
  const index = appendIndex(slotsOf(racks, channel), item.slotId)
  return index === null ? null : { kind: 'move', index }
}

// plugins dropped on the mixer's strips: an effect from the sidebar goes at
// the end of that channel's chain, a row of the Effects Rack moves there,
// and an instrument starts a pattern of it on that track (see
// addSynthOnTrack). returns the channel what's dragged would go on, to show
// it
export const useChannelDrop = (ref: React.RefObject<HTMLElement>): ChannelId | null => {
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const [target, setTarget] = React.useState<ChannelId | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    if(!isChannelItem(item)) return

    const channel = channelAt(ref.current, clientX, clientY)
    const drop = channel === null ? null : channelDrop(store.getState().effects.racks, channel, item)
    if(type === 'move') {
      setTarget(drop ? channel : null)
      return
    }

    setTarget(null)
    if(type !== 'drop' || channel === null || !drop) return

    if(item.kind === 'instrument' && channel !== 'master') {
      dispatch(addSynthOnTrack(item.pluginId, channel)).catch(() => {})
    } else if(item.kind === 'effect') {
      dispatch(dropEffectOnChannel(channel, item.pluginId)).catch(() => {})
    } else if(item.kind === 'slot' && drop.kind === 'move') {
      dispatch(moveEffect(item.slotId, channel, drop.index)).catch(() => {})
    }
  }), [dispatch, ref, store])

  return target
}
