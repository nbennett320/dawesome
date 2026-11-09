import React from 'react'
import { AnalogButton, Led } from '../fourtrack'

interface Props {
  label: string
  on: boolean
  onToggle: () => void
  children: React.ReactNode
}

// a small latching key with a blue LED above it, for the topbar's toggles
const LatchKey = ({ label, on, onToggle, children }: Props) => (
  <div className='flex flex-col items-center gap-1'>
    <Led color='blue' state={on ? 'on' : 'off'} />
    <AnalogButton size='xs' latched={on} aria-label={label} title={label} onPress={onToggle}>
      {children}
    </AnalogButton>
  </div>
)

export default LatchKey
