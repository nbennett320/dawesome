import React from 'react'
import { SnapNote } from '../../editing/snap'
import styles from '../topbar/styles.module.scss'

// snap lengths, labelled in bars and beats (one beat is a quarter note)
const SNAP_OPTIONS: Array<{ note: SnapNote, label: string }> = [
  { note: 'WholeNote', label: '1 bar' },
  { note: 'HalfNote', label: '1/2 bar' },
  { note: 'QuarterNote', label: '1 beat' },
  { note: 'EighthNote', label: '1/2 beat' },
  { note: 'SixteenthNote', label: '1/4 beat' },
]

interface Props {
  note: SnapNote
  enabled: boolean
  onNote: (note: SnapNote) => void
  // what a screen reader calls it: "Playlist snap amount"
  label: string
}

// how far apart a view's snap lines are
const SnapInput = ({ note, enabled, onNote, label }: Props) => (
  <div className='flex flex-row items-center gap-1.5'>
    <span className='ft-label'>Snap</span>
    <select
      value={note}
      // still selectable while snap is off, but dimmed to show it isn't applied
      className={`${styles.SnapInputField} ft-readout text-sm py-0.5 ${enabled ? '' : 'opacity-50'}`}
      aria-label={label}
      onChange={e => onNote(e.currentTarget.value as SnapNote)}
    >
      {SNAP_OPTIONS.map(({ note: value, label: text }) => (
        <option key={value} value={value}>{text}</option>
      ))}
    </select>
  </div>
)

export default SnapInput
