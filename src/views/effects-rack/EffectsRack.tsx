import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  fetchEffects, selectChannelSlots, selectExpandedSlotId, selectRackChannel,
} from '../../state/slices/effectsSlice'
import { selectViewShowing } from '../../state/slices/windowSlice'
import { dockable } from '../../types/effects'
import { View } from '../../types/ui'
import RackExpanded from './RackExpanded'
import RackStack from './RackStack'

// one channel's effects, in the order the sound goes through them: the
// stack of them, or one expanded with its own interface docked in the tab
const EffectsRack = () => {
  const dispatch = useAppDispatch()
  const channel = useAppSelector(selectRackChannel)
  const slots = useAppSelector(selectChannelSlots(channel))
  const expandedId = useAppSelector(selectExpandedSlotId)
  // tabs stay mounted while hidden: a docked interface is only shown while
  // this one is its pane's selected tab
  const showing = useAppSelector(selectViewShowing(View.EffectsRack))

  // the engine owns the chains: catch up with it when the tab opens
  React.useEffect(() => {
    dispatch(fetchEffects()).catch(e => {
      // eslint-disable-next-line no-console
      console.error('could not read the effects:', e)
    })
  }, [dispatch])

  // (an expanded slot that's gone, or has nothing to show, is the stack)
  const expanded = dockable(slots).find(slot => slot.id === expandedId)

  if(expanded) {
    return <RackExpanded channel={channel} slots={slots} slot={expanded} active={showing} />
  }
  return <RackStack channel={channel} slots={slots} />
}

export default EffectsRack
