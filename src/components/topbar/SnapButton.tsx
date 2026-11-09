import React from 'react'
import { toggleSnap, selectSnapEnabled } from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import RulerIcon from '../icons/RulerIcon'
import LatchKey from './LatchKey'

const SnapButton = () => {
  const snap = useAppSelector(selectSnapEnabled)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Snap to grid' on={snap} onToggle={() => dispatch(toggleSnap())}>
      <RulerIcon fill='currentColor' />
    </LatchKey>
  )
}

export default SnapButton
