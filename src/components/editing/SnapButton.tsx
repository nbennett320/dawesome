import React from 'react'
import LatchKey from '../topbar/LatchKey'
import { RulerIcon } from '../icons'

interface Props {
  enabled: boolean
  onToggle: () => void
}

// whether what's moved, trimmed, cut and drawn in a view lands on the grid
const SnapButton = ({ enabled, onToggle }: Props) => (
  <LatchKey label='Snap to grid' on={enabled} onToggle={onToggle}>
    <RulerIcon />
  </LatchKey>
)

export default SnapButton
