import React from 'react'
import { NameInput } from '../../common'

interface Props {
  // the name it has now
  initial: string
  // read out by screen readers, e.g. "Folder name"
  label: string
  depth?: number
  maxLength?: number
  // the new name, or null when it's left as it was (escape, or unchanged)
  onDone: (name: string | null) => void
}

// a row's name, being changed in place. enter (or clicking away) keeps it
const RenameField = ({ initial, label, depth = 0, maxLength = 120, onDone }: Props) => (
  <div
    className='h-6 flex flex-row items-center pr-3 border-l-2 border-accent bg-accent-tint'
    style={{ paddingLeft: depth * 12 + 8 }}
  >
    <NameInput
      initial={initial}
      aria-label={label}
      maxLength={maxLength}
      className='w-full h-5'
      onDone={name => {
        const trimmed = name?.trim()
        onDone(trimmed && trimmed !== initial ? trimmed : null)
      }}
    />
  </div>
)

export default RenameField
