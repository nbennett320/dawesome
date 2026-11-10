import React from 'react'
import PlayPauseButton from './PlayPauseButton'
import RecordButton from './RecordButton'
import StopButton from './StopButton'
import MetronomeButton from './MetronomeButton'
import CountInButton from './CountInButton'
import LoopButton from './LoopButton'

// the transport keys, in the Transport tab's order, then the toggles, each
// set in a recessed tray
const ButtonArea = () => (
  <div className='flex flex-row items-center gap-2'>
    <div className='ft-tray rounded-md flex flex-row items-center gap-1.5 px-2 py-1.5'>
      <RecordButton />
      <PlayPauseButton />
      <StopButton />
    </div>
    <div className='ft-tray rounded-md flex flex-row items-end gap-1.5 px-2 py-1'>
      <MetronomeButton />
      <CountInButton />
      <LoopButton />
    </div>
  </div>
)

export default ButtonArea
