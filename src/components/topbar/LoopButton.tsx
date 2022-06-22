import React from 'react'
import {
  toggleLoop,
  selectLoopEnabled,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import Button from '../common/Button'
import LoopIcon from '../icons/LoopIcon'
import styles from './styles.module.scss'

const LoopButton = () => {
  const loop = useAppSelector(selectLoopEnabled)
  const dispatch = useAppDispatch()

  return (
    <div className={`${styles.LoopButton} row-auto`}>
      <Button
        onClick={() => dispatch(toggleLoop())}
        aria-label="Loop Button"
        className={`${loop ? 'bg-blue-400 hover:bg-blue-300' : 'bg-gray-300 hover:bg-gray-400' } py-1 px-3 rounded inline-flex items-center`}
      >
        <LoopIcon fill="white" />
      </Button>
    </div>
  )
}

export default LoopButton
