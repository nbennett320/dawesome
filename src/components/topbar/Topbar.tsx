import React from 'react'
import TopbarBase from './TopbarBase'
import ButtonArea from './ButtonArea'
import TempoInput from './TempoInput'
import SnapInput from './SnapInput'
import TopbarTimer from './TopbarTimer'
import styles from './styles.module.scss'

const Topbar = () => (
  <TopbarBase>
    <div className={`${styles.Topbar} items-center gap-6 py-2`}>
      <ButtonArea />
      <div className='flex flex-row items-end gap-4'>
        <TempoInput />
        <SnapInput />
      </div>
      <TopbarTimer />
    </div>
  </TopbarBase>
)

export default Topbar
