import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectRacks, showRackChannel } from '../../state/slices/effectsSlice'
import { selectPlaylistUI } from '../../state/slices/playlistSlice'
import { ChannelId, channelKey, channelLabel, parseChannelKey, Racks } from '../../types/effects'

// the tracks to choose from: the Mixer's (one per playlist track), plus any
// further one that has effects, or is being shown
export const channelChoices = (
  trackCount: number,
  names: Record<number, string>,
  racks: Racks,
  shown: ChannelId,
): Array<{ value: string, label: string }> => {
  const highest = Math.max(
    trackCount - 1,
    ...racks.map(rack => rack.track ?? -1),
    shown === 'master' ? -1 : shown,
  )

  return [
    ...Array.from({ length: highest + 1 }, (_, track) => ({
      value: channelKey(track),
      label: channelLabel(names, track),
    })),
    { value: channelKey('master'), label: channelLabel(names, 'master') },
  ]
}

// which channel the rack shows
const ChannelSelect = ({ channel }: { channel: ChannelId }) => {
  const dispatch = useAppDispatch()
  const { trackCount } = useAppSelector(selectPlaylistUI)
  const names = useAppSelector(state => state.mixer.names)
  const racks = useAppSelector(selectRacks)
  const choices = channelChoices(trackCount, names, racks, channel)

  return (
    <select
      className='ft-readout text-xs py-1 text-left w-36 min-w-0 font-sans'
      aria-label='Channel'
      title='The channel whose effects are shown'
      value={channelKey(channel)}
      onChange={e => {
        const picked = parseChannelKey(e.currentTarget.value)
        if(picked !== null) dispatch(showRackChannel(picked))
      }}
    >
      {choices.map(choice => <option key={choice.value} value={choice.value}>{choice.label}</option>)}
    </select>
  )
}

export default ChannelSelect
