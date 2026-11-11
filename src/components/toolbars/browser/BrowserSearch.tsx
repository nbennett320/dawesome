import React from 'react'

interface Props {
  value: string
  onChange: (value: string) => void
  // read out by screen readers, e.g. "Search samples"
  label: string
  // buttons beside the field, e.g. a new folder
  children?: React.ReactNode
}

// the search field at the top of each sidebar browser. escape clears it
const BrowserSearch = ({ value, onChange, label, children }: Props) => (
  <div className='px-3 pb-2 pt-4 flex flex-row items-center gap-1.5'>
    <input
      type='search'
      placeholder='Search'
      aria-label={label}
      className='ft-readout flex-1 min-w-0 text-left text-xs px-2 py-1 font-sans placeholder:text-ink-faint'
      value={value}
      onChange={e => onChange(e.currentTarget.value)}
      onKeyDown={e => { if(e.key === 'Escape') onChange('') }}
    />
    {children}
  </div>
)

export default BrowserSearch
