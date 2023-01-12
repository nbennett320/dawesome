import React from 'react'
import { invoke } from '@tauri-apps/api'
import {
  selectPlaylistPlaying,
} from '../../state/slices/playlistSlice'
import Button from '../common/Button'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import RecordIcon from '../icons/RecordIcon'

const RecordButton = () => {
  const recording = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  const toggleRecord = () => {
    invoke('toggle_record_input', {})
  }
  
  return (
    <Button
      onClick={toggleRecord}
      className="bg-gray-400 hover:bg-gray-500 py-1 px-3 rounded inline-flex items-center"
      aria-label="Play/Pause Button"
    >
      <RecordIcon />
    </Button>
  )
}

export default RecordButton