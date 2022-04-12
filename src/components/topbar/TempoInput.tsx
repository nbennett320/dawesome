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
import MetronomeIcon from '../icons/MetronomeIcon'
import styles from './styles.module.scss'

const TempoInput = () => {
  const tempo = useAppSelector(selectPlaylistTempo)
  const metronome = useAppSelector(selectMetronomeEnabled)
  const dispatch = useAppDispatch()

  return (
    <div className={`${styles.TempoInput}`}>
      <div className={`${styles.TempoInputFieldContainer}`}>
        <Input
          onInput={(e) =>
            dispatch(setPlaylistTempo(parseFloat(e.currentTarget.value)))
          }
          value={tempo}
          type="number"
          label="Tempo"
          className={`${styles.TempoInputField} text-gray-600 text-sm rounded`}
          aria-label="Playlist tempo input"
        />
      </div>

      <div className={`${styles.MetronomeButton} row-auto`}>
        <Button
          onClick={() => dispatch(toggleMetronome())}
          aria-label="Play/Pause Button"
          className={`${metronome ? 'bg-blue-400 hover:bg-blue-300' : 'bg-gray-300 hover:bg-gray-400' } py-1 px-3 rounded inline-flex items-center`}
        >
          <MetronomeIcon fill="white" />
        </Button>
      </div>
    </div>
  )
}

export default TempoInput
