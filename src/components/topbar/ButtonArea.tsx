import React from 'react'
import PlayPauseButton from './PlayPauseButton'
import MetronomeButton from './MetronomeButton'
import LoopButton from './LoopButton'
import SnapButton from './SnapButton'
import styles from './styles.module.scss'

const ButtonArea = () => (
  <div className={`${styles.ButtonArea}`}>
    <PlayPauseButton />
    <MetronomeButton />
    <LoopButton />
    <SnapButton />
  </div>
)

export default ButtonArea
