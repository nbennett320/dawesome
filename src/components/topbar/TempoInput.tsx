import React from 'react'
import { 
  setPlaylistTempo,
  selectPlaylistTempo,
  toggleMetronome,
  selectMetronomeEnabled,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import Input from '../common/Input'
import Button from '../common/Button'

const TempoInput = () => {
  const tempo = useAppSelector(selectPlaylistTempo) 
  const metronome = useAppSelector(selectMetronomeEnabled) 
  const dispatch = useAppDispatch()

  return (
    <div>
      <Input 
        onInput={(e) => dispatch(setPlaylistTempo(parseFloat(e.currentTarget.value)))}
        value={tempo}
        type='number'
        aria-label='Playlist tempo input'
      />
      <div className='row-auto'>
        <span>Metronome </span>
        <Button 
          onClick={() => dispatch(toggleMetronome())}
          aria-label="Play/Pause Button"
        > 
          {metronome ? <>Enabled</> : <>Disabled</>}
        </Button>
      </div>
    </div>
  )
}

export default TempoInput