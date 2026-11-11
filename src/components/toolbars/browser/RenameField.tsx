import React from 'react'

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
const RenameField = ({ initial, label, depth = 0, maxLength = 120, onDone }: Props) => {
  const [draft, setDraft] = React.useState(initial)
  const done = React.useRef(false)

  const finish = (name: string | null) => {
    if(done.current) return
    done.current = true
    const trimmed = name?.trim()
    onDone(trimmed && trimmed !== initial ? trimmed : null)
  }

  return (
    <div
      className='h-6 flex flex-row items-center pr-3 border-l-2 border-accent bg-accent-tint'
      style={{ paddingLeft: depth * 12 + 8 }}
    >
      <input
        type='text'
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        aria-label={label}
        maxLength={maxLength}
        className='ft-scribble w-full h-5 text-xs px-1 outline-none ring-2 ring-accent-focus'
        value={draft}
        onFocus={e => e.currentTarget.select()}
        onChange={e => setDraft(e.currentTarget.value)}
        onBlur={() => finish(draft)}
        onKeyDown={e => {
          if(e.key === 'Enter') finish(draft)
          if(e.key === 'Escape') finish(null)
        }}
      />
    </div>
  )
}

export default RenameField
