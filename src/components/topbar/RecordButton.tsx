import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  collectRecording,
  selectAnyArmed,
  selectRecording,
  toggleRecording,
} from '../../state/slices/recordingSlice'
import { AnalogButton, RecordIcon } from '../fourtrack'

// how often a recording is checked on, to catch it ending some other way
const CHECK_EVERY_MS = 250

// records every armed track from the playhead, held down while it records
const RecordButton = () => {
  const dispatch = useAppDispatch()
  const recording = useAppSelector(selectRecording)
  const anyArmed = useAppSelector(selectAnyArmed)

  // stopping playback (or a loop coming round) ends the recording too
  React.useEffect(() => {
    if(!recording) return undefined
    const timer = setInterval(() => { dispatch(collectRecording()) }, CHECK_EVERY_MS)
    return () => clearInterval(timer)
  }, [dispatch, recording])

  let title = 'Record every armed track from the playhead'
  if(recording) title = 'Stop recording'
  else if(!anyArmed) title = 'Record (arm a track first, with the R key on its mixer strip)'

  return (
    <AnalogButton
      size='sm'
      aria-label={recording ? 'Stop recording' : 'Record'}
      title={title}
      latched={recording}
      onPress={() => { dispatch(toggleRecording()) }}
    >
      <RecordIcon className='ft-icon-rec' />
    </AnalogButton>
  )
}

export default RecordButton
