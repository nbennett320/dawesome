import React from 'react'
import TopbarBase from './TopbarBase'
import PlayPauseButton from './PlayPauseButton'
import TempoInput from './TempoInput'
import TopbarTimer from './TopbarTimer'

interface Props {
 
}

const Topbar = (props: Props) => (
  <TopbarBase>
    <PlayPauseButton />
    <TempoInput />
    <TopbarTimer />
  </TopbarBase>
)

export default Topbar
