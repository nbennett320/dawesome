import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectRecording, toggleRecording } from '../../state/slices/recordingSlice'
import { AnalogButton, RecordIcon } from '../fourtrack'

// records a take from the input into the project's recordings folder, held
// down while it records
const RecordButton = () => {
  const dispatch = useAppDispatch()
  const recording = useAppSelector(selectRecording)

  return (
    <AnalogButton
      size='sm'
      aria-label={recording ? 'Stop recording' : 'Record'}
      title={recording ? 'Stop recording' : 'Record'}
      latched={recording}
      onPress={() => { dispatch(toggleRecording()) }}
    >
      <RecordIcon className='ft-icon-rec' />
    </AnalogButton>
  )
}

export default RecordButton
