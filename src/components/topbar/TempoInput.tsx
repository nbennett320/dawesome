import React from 'react'
import {
  setPlaylistTempo,
  selectPlaylistTempo,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import Input from '../common/Input'
import styles from './styles.module.scss'

const TempoInput = () => {
  const tempo = useAppSelector(selectPlaylistTempo)
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
    </div>
  )
}

export default TempoInput
