import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from 'state/store'
import { useAppDispatch } from '../../hooks/redux'
import { isEffectItem, onSampleDrag } from '../../state/sampleDrag'
import { dropEffectOnChannel, moveEffect } from '../../state/slices/effectsSlice'
import { appendIndex, ChannelId, parseChannelKey, slotsOf } from '../../types/effects'

// marks a mixer strip as a place effects can be dropped: its channel's key
// (see channelKey)
export const DROP_CHANNEL = 'data-drop-channel'

// the channel under the mouse, if it's a strip inside `box`
export const channelAt = (box: HTMLElement | null, clientX: number, clientY: number): ChannelId | null => {
  const under = document.elementFromPoint(clientX, clientY)
  if(!box || !under || !box.contains(under)) return null
  return parseChannelKey(under.closest(`[${DROP_CHANNEL}]`)?.getAttribute(DROP_CHANNEL))
}

// effects dropped on the mixer's strips: one from the sidebar goes at the
// end of that channel's chain, and a row of the Effects Rack moves there.
// returns the channel what's dragged would go on, to show it
export const useChannelDrop = (ref: React.RefObject<HTMLElement>): ChannelId | null => {
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const [target, setTarget] = React.useState<ChannelId | null>(null)

  React.useEffect(() => onSampleDrag(({ type, item, clientX, clientY }) => {
    if(!isEffectItem(item)) return

    const channel = channelAt(ref.current, clientX, clientY)
    if(type === 'move') {
      setTarget(channel)
      return
    }

    setTarget(null)
    if(type !== 'drop' || channel === null) return

    if(item.kind === 'effect') {
      dispatch(dropEffectOnChannel(channel, item.pluginId)).catch(() => {})
      return
    }

    // a row of the rack: to the end of this channel's chain, settings kept
    const index = appendIndex(slotsOf(store.getState().effects.racks, channel), item.slotId)
    if(index !== null) dispatch(moveEffect(item.slotId, channel, index)).catch(() => {})
  }), [dispatch, ref, store])

  return target
}
