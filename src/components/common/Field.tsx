import React from 'react'

// a label over a control
const Field = ({ label, htmlFor, children }: { label: string, htmlFor: string, children: React.ReactNode }) => (
  <div className='flex flex-col gap-1'>
    <label htmlFor={htmlFor} className='ft-label'>{label}</label>
    {children}
  </div>
)

export default Field
