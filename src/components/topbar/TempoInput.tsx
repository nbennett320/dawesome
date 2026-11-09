import React from 'react'
import { setPlaylistTempo, selectPlaylistTempo } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

const TempoInput = () => {
  const tempo = useAppSelector(selectPlaylistTempo)
  const dispatch = useAppDispatch()

  return (
    <div className='flex flex-row items-center gap-1.5'>
      <span className='ft-label'>Tempo</span>
      <input
        type='number'
        className='ft-readout text-sm py-0.5 w-14'
        aria-label='Playlist tempo input'
        value={tempo}
        onInput={e => dispatch(setPlaylistTempo(parseFloat(e.currentTarget.value)))}
      />
    </div>
  )
}

export default TempoInput
