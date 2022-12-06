import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useAppDispatch } from '../../hooks/redux'
import SampleDetailsCanvas from './SampleDetailsCanvas'
import styles from './styles.module.scss'

const SampleDetails = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const dispatch = useAppDispatch()

  return (
    <div 
      ref={ref}
      className={`${styles.SampleDetails} h-full`}
    >
      <SampleDetailsCanvas />
    </div>
  )
}

export default SampleDetails
