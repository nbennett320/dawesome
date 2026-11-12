import React from 'react'
import { cx } from '../../util/helpers'

type Props = Omit<
  React.InputHTMLAttributes<HTMLInputElement>,
  'type' | 'inputMode' | 'value' | 'onChange' | 'onFocus' | 'onBlur' | 'onKeyDown'
> & {
  // what it reads until something is typed
  value: string
  // what was typed, once it's done: enter, or clicking away. escape drops it
  onCommit: (typed: string) => void
}

// a readout that can be typed into, for a number. the owner decides what the
// text means (and whether it's good enough to keep)
const ReadoutInput = ({ value, onCommit, className, ...rest }: Props) => {
  const [draft, setDraft] = React.useState<string | null>(null)

  const commit = () => {
    if(draft === null) return
    onCommit(draft)
    setDraft(null)
  }

  return (
    <input
      type='text'
      inputMode='decimal'
      className={cx('ft-readout', className)}
      value={draft ?? value}
      onFocus={e => e.currentTarget.select()}
      onChange={e => setDraft(e.currentTarget.value)}
      onBlur={commit}
      onKeyDown={e => {
        if(e.key === 'Enter') e.currentTarget.blur()
        if(e.key === 'Escape') setDraft(null)
      }}
      {...rest}
    />
  )
}

export default ReadoutInput
