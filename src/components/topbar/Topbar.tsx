import React from 'react'
import TopbarBase from './TopbarBase'
import PlayPauseButton from './PlayPauseButton'
import TempoInput from './TempoInput'
import TopbarTimer from './TopbarTimer'

const Topbar = () => (
  <TopbarBase>
    <PlayPauseButton />
    <TempoInput />
    <TopbarTimer />
  </TopbarBase>
)

export default Topbar
