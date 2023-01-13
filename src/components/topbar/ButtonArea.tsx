import React from 'react'
import PlayPauseButton from './PlayPauseButton'
import RecordButton from './RecordButton'
import MetronomeButton from './MetronomeButton'
import LoopButton from './LoopButton'
import SnapButton from './SnapButton'
import styles from './styles.module.scss'

const ButtonArea = () => (
  <div className={`w-64 ${styles.ButtonArea}`}>
    <div className='flex flex-row justify-between'>
      <PlayPauseButton className='rounded-tr-none rounded-br-none' />
      <RecordButton className='rounded-tl-none rounded-bl-none' />
    </div>
    <div className='flex flex-row ml-2.5 mt-2.5 mb-auto'>
      <MetronomeButton className='rounded-tr-none rounded-br-none' height={16} width={16} />
      <LoopButton className='rounded-tr-none rounded-br-none rounded-tl-none rounded-bl-none' height={16} width={16}/>
      <SnapButton className='rounded-tl-none rounded-bl-none' height={16} width={16}/>
    </div>
  </div>
)

export default ButtonArea
