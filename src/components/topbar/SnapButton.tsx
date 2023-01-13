import React from 'react'
import {
  toggleSnap,
  selectSnapEnabled,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import Button from '../common/Button'
import RulerIcon from '../icons/RulerIcon'
import styles from './styles.module.scss'

interface Props {
  className?: string
  height?: number
  width?: number
}

const SnapButton = (props: Props) => {
  const snap = useAppSelector(selectSnapEnabled)
  const dispatch = useAppDispatch()

  return (
    <div className={`${styles.SnapButton} row-auto`}>
      <Button
        onClick={() => dispatch(toggleSnap())}
        aria-label="Loop Button"
        className={`${snap ? 'bg-blue-400 hover:bg-blue-300' : 'bg-gray-300 hover:bg-gray-400' } py-1 px-3 rounded inline-flex items-center ${props.className}`}
      >
        <RulerIcon fill="white" height={props.height} width={props.width} />
      </Button>
    </div>
  )
}

export default SnapButton
