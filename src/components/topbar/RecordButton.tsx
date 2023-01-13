import React from 'react'
import { invoke } from '@tauri-apps/api'
import {
  selectPlaylistPlaying,
} from '../../state/slices/playlistSlice'
import Button from '../common/Button'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import RecordIcon from '../icons/RecordIcon'

interface Props {
  className?: string
}

const RecordButton = (props: Props) => {
  const recording = useAppSelector(selectPlaylistPlaying)
  const dispatch = useAppDispatch()

  const toggleRecord = () => {
    invoke('toggle_record_input', {})
  }
  
  return (
    <Button
      onClick={toggleRecord}
      className={`bg-gray-400 hover:bg-gray-500 py-1 px-3 rounded inline-flex items-center ${props.className}`}
      aria-label="Record Button"
    >
      <RecordIcon fill="white" />
    </Button>
  )
}

export default RecordButton