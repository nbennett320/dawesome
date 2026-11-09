import React from 'react'

interface Props {
  value: string
  onChange: (value: string) => void
  // read out by screen readers, e.g. "Search samples"
  label: string
}

// the search field at the top of each sidebar browser. escape clears it
const BrowserSearch = ({ value, onChange, label }: Props) => (
  <div className='px-3 pb-2 pt-4'>
    <input
      type='search'
      placeholder='Search'
      aria-label={label}
      className='ft-readout w-full text-left text-xs px-2 py-1 font-sans placeholder:text-slate-400'
      value={value}
      onChange={e => onChange(e.currentTarget.value)}
      onKeyDown={e => { if(e.key === 'Escape') onChange('') }}
    />
  </div>
)

export default BrowserSearch
