import React from 'react'
import TopbarBase from './TopbarBase'
import PlayPauseButton from './PlayPauseButton'
import TempoInput from './TempoInput'
import TopbarTimer from './TopbarTimer'
import styles from './styles.module.scss'

const Topbar = () => (
  <TopbarBase>
    <div className={`${styles.Topbar} py-2`}>
      <PlayPauseButton />
      <TempoInput />
      <TopbarTimer />
    </div>
  </TopbarBase>
)

export default Topbar
