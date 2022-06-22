import React from 'react'
import TopbarBase from './TopbarBase'
import ButtonArea from './ButtonArea'
import TempoInput from './TempoInput'
import TopbarTimer from './TopbarTimer'
import styles from './styles.module.scss'

const Topbar = () => (
  <TopbarBase>
    <div className={`${styles.Topbar} py-2`}>
      <ButtonArea />
      <TempoInput />
      <TopbarTimer />
    </div>
  </TopbarBase>
)

export default Topbar
