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
    <LatchKey label='Count in a bar before recording'
      hint={countIn ? 'Count-in is on: recording starts after one bar. Click to turn it off' : 'Count-in is off. Click to count in one bar before recording starts'}
      on={countIn} onToggle={() => dispatch(toggleCountIn())}>
      <CountInIcon />
    </LatchKey>
  )
}

export default CountInButton
