import React from 'react'
import { 
  setPlaylistTempo,
  selectPlaylistTempo,
  PlaylistTempoState
} from '../../state/slices/playlistTempoSlice'
import {
  toggleMetronome,
  selectMetronomeEnabled,
  PlaylistMetronomeEnabledState
} from '../../state/slices/playlistMetronomeEnabledSlice'
import Input from '../common/Input'
import Button from '../common/Button'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

const TempoInput = () => {
  const tempo = useAppSelector<PlaylistTempoState>(selectPlaylistTempo) 
  const metronome = useAppSelector<PlaylistMetronomeEnabledState>(selectMetronomeEnabled) 
  const dispatch = useAppDispatch()

  return (
    <div>
      <Input 
        onInput={(e) => dispatch(setPlaylistTempo(parseFloat(e.currentTarget.value)))}
        value={tempo.value}
        type='number'
        aria-label='Playlist tempo input'
      />
      <div className='row-auto'>
        <span>Metronome </span>
        <Button 
          onClick={() => dispatch(toggleMetronome())}
          aria-label="Play/Pause Button"
        > 
          {metronome.value ? <>Enabled</> : <>Disabled</>}
        </Button>
      </div>
    </div>
  )
}

export default TempoInput