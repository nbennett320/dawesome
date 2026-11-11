import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { fetchEffects, selectChannelSlots, selectRackChannel } from '../../state/slices/effectsSlice'
import RackStack from './RackStack'

// one channel's effects, in the order the sound goes through them. each
// one's own interface opens in a tab of its own (see views/plugin)
const EffectsRack = () => {
  const dispatch = useAppDispatch()
  const channel = useAppSelector(selectRackChannel)
  const slots = useAppSelector(selectChannelSlots(channel))

  // the engine owns the chains: catch up with it when the tab opens
  React.useEffect(() => {
    dispatch(fetchEffects()).catch(e => {
      // eslint-disable-next-line no-console
      console.error('could not read the effects:', e)
    })
  }, [dispatch])

  return <RackStack channel={channel} slots={slots} />
}

export default EffectsRack
