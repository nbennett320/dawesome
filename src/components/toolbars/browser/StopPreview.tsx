import React from 'react'
import { StopIcon } from '../../icons'

// stops the preview, in the slot at the start of a row (a folder's chevron,
// a take's dot). it's inside the row's button, so it's a span, and pressing
// it doesn't play or drag the row again. from the keyboard, Escape on the
// row stops it instead (so it's not a tab stop of its own)
const StopPreview = ({ onStop }: { onStop: () => void }) => (
  <span
    role='button'
    tabIndex={-1}
    aria-label='Stop preview'
    title='Stop preview'
    className='w-3 h-3 shrink-0 flex items-center justify-center text-accent hover:text-ink-strong cursor-pointer'
    onMouseDown={ev => {
      ev.stopPropagation()
      ev.preventDefault()
      if(ev.button === 0) onStop()
    }}
  >
    <StopIcon className='w-3 h-3' />
  </span>
)

export default StopPreview
