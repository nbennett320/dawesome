import React from 'react'
import { cx } from '../../util/helpers'

type Props = Omit<
  React.InputHTMLAttributes<HTMLInputElement>,
  'type' | 'value' | 'defaultValue' | 'onChange' | 'onFocus' | 'onBlur' | 'onKeyDown' | 'autoFocus'
> & {
  // the name it has now
  initial: string
  // the new text, when enter is pressed or focus leaves, or null on escape.
  // called once
  onDone: (text: string | null) => void
}

// a name being changed in place, with the cursor in it. size it with a class
const NameInput = ({ initial, onDone, className, ...rest }: Props) => {
  const [draft, setDraft] = React.useState(initial)
  const done = React.useRef(false)

  const finish = (text: string | null) => {
    if(done.current) return
    done.current = true
    onDone(text)
  }

  return (
    <input
      type='text'
      // eslint-disable-next-line jsx-a11y/no-autofocus
      autoFocus
      className={cx('ft-scribble text-xs px-1 outline-none ring-2 ring-accent-focus', className)}
      value={draft}
      onFocus={e => e.currentTarget.select()}
      onChange={e => setDraft(e.currentTarget.value)}
      onBlur={() => finish(draft)}
      onKeyDown={e => {
        if(e.key === 'Enter') finish(draft)
        if(e.key === 'Escape') finish(null)
      }}
      {...rest}
    />
  )
}

export default NameInput
