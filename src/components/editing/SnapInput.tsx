import React from 'react'
import { SNAP_OPTIONS, SnapNote } from '../../editing/snap'
import styles from '../topbar/styles.module.scss'

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
