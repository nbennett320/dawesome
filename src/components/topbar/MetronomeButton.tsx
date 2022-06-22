import React from 'react'
import {
  toggleMetronome,
  selectMetronomeEnabled,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import Button from '../common/Button'
import MetronomeIcon from '../icons/MetronomeIcon'
import styles from './styles.module.scss'

const MetronomeButton = () => {
  const metronome = useAppSelector(selectMetronomeEnabled)
  const dispatch = useAppDispatch()

  return (
    <div className={`${styles.MetronomeButton} row-auto`}>
      <Button
        onClick={() => dispatch(toggleMetronome())}
        aria-label="Play/Pause Button"
        className={`${metronome ? 'bg-blue-400 hover:bg-blue-300' : 'bg-gray-300 hover:bg-gray-400' } py-1 px-3 rounded inline-flex items-center`}
      >
        <MetronomeIcon fill="white" />
      </Button>
    </div>
  )
}

export default MetronomeButton
