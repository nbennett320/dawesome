import React from 'react'
import RulerIcon from '../icons/RulerIcon'
import LatchKey from '../topbar/LatchKey'

interface Props {
  enabled: boolean
  onToggle: () => void
}

// whether what's moved, trimmed, cut and drawn in a view lands on the grid
const SnapButton = ({ enabled, onToggle }: Props) => (
  <LatchKey label='Snap to grid' on={enabled} onToggle={onToggle}>
    <RulerIcon fill='currentColor' />
  </LatchKey>
)

export default SnapButton
