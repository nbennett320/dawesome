import React from 'react'
import Field from './Field'

// a labelled dropdown
const Choice = ({ id, label, value, options, disabled, onChange, className = 'w-72 max-w-full' }: {
  id: string
  label: string
  value: string
  options: Array<{ value: string, label: string }>
  disabled?: boolean
  onChange: (value: string) => void
  // the width, by default
  className?: string
}) => (
  <Field label={label} htmlFor={id}>
    <select
      id={id}
      className={`ft-readout text-sm py-1 text-left ${className}`}
      value={value}
      disabled={disabled}
      onChange={e => onChange(e.currentTarget.value)}
    >
      {options.map(option => (
        <option key={option.value} value={option.value}>{option.label}</option>
      ))}
    </select>
  </Field>
)

export default Choice
