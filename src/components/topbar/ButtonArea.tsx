import React from 'react'
import PlayPauseButton from './PlayPauseButton'
import RecordButton from './RecordButton'
import MetronomeButton from './MetronomeButton'
import LoopButton from './LoopButton'
import SnapButton from './SnapButton'

// the transport keys, then the toggles, each set in a recessed tray
const ButtonArea = () => (
  <div className='flex flex-row items-center gap-3'>
    <div className='ft-tray rounded-lg flex flex-row items-end gap-2 px-2.5 pt-2.5 pb-1.5'>
      <PlayPauseButton />
      <RecordButton />
    </div>
    <div className='ft-tray rounded-lg flex flex-row items-end gap-2 px-2.5 pt-2 pb-1.5'>
      <MetronomeButton />
      <LoopButton />
      <SnapButton />
    </div>
  </div>
)

export default ButtonArea
