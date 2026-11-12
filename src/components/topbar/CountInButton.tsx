import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectCountIn, toggleCountIn } from '../../state/slices/recordingSlice'
import LatchKey from './LatchKey'
import { CountInIcon } from '../icons'

// whether recording counts in a bar of clicks first
const CountInButton = () => {
  const countIn = useAppSelector(selectCountIn)
  const dispatch = useAppDispatch()

  return (
    <LatchKey label='Count in a bar before recording' on={countIn} onToggle={() => dispatch(toggleCountIn())}>
      <CountInIcon />
    </LatchKey>
  )
}

export default CountInButton
