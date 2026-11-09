import React from 'react'
import {
  SnapNote,
  setSnapSubdivision,
  selectSnapEnabled,
  selectSnapNote,
} from '../../state/slices/playlistSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import styles from './styles.module.scss'

// snap lengths, labelled in bars and beats (one beat is a quarter note)
const SNAP_OPTIONS: Array<{ note: SnapNote, label: string }> = [
  { note: 'WholeNote', label: '1 bar' },
  { note: 'HalfNote', label: '1/2 bar' },
  { note: 'QuarterNote', label: '1 beat' },
  { note: 'EighthNote', label: '1/2 beat' },
  { note: 'SixteenthNote', label: '1/4 beat' },
]

const SnapInput = () => {
  const note = useAppSelector(selectSnapNote)
  const enabled = useAppSelector(selectSnapEnabled)
  const dispatch = useAppDispatch()

  return (
    <div className='flex flex-row items-center gap-1.5'>
      <span className='ft-label'>Snap</span>
      <select
        value={note}
        // still selectable while snap is off, but dimmed to show it isn't applied
        className={`${styles.SnapInputField} ft-readout text-sm py-0.5 ${enabled ? '' : 'opacity-50'}`}
        aria-label="Playlist snap amount"
        onChange={(e) =>
          dispatch(setSnapSubdivision(e.currentTarget.value as SnapNote))
        }
      >
        {SNAP_OPTIONS.map(({ note: value, label }) => (
          <option key={value} value={value}>{label}</option>
        ))}
      </select>
    </div>
  )
}

export default SnapInput
