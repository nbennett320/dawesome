import React from 'react'

interface Props extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  // the row heard or shown last
  selected: boolean
  // whether the project uses it: shows a dot when it does, and dims it when
  // it doesn't. left out, the row has no room for the dot
  used?: boolean
  // dragged somewhere (a sample, a pattern), rather than opened (a folder)
  grab?: boolean
}

// marks what the project uses, so the rest stands apart
// (in a slot as wide as a folder's chevron, so names line up in a tree)
const UsedDot = ({ used }: { used: boolean }) => (
  <span className='w-3 shrink-0 flex justify-center'>
    <span
      className={`w-1.5 h-1.5 rounded-full ${used ? 'bg-sky-500' : 'bg-transparent'}`}
      role='img'
      aria-label={used ? 'In use' : 'Not used'}
    />
  </span>
)

// a row in one of the sidebar's browsers. the same look everywhere, so a
// sample, a pattern and a take all read alike
// - give it a role (treeitem, option) for aria-selected to mean something
const BrowserRow = ({
  selected, used, grab = true, className = '', children, ...rest
}: React.PropsWithChildren<Props>) => {
  let tone = 'text-slate-600'
  if(used === false) tone = 'text-slate-400'

  return (
    // eslint-disable-next-line jsx-a11y/role-supports-aria-props -- callers give it a role
    <button
      type='button'
      aria-selected={selected}
      className={`group w-full h-6 flex flex-row items-center gap-1.5 pl-2 pr-3 text-left text-xs outline-none
        border-l-2 focus-visible:bg-sky-50 ${
          selected
            ? 'border-sky-500 bg-sky-50 text-slate-800'
            : `border-transparent hover:bg-slate-100 hover:text-slate-800 ${tone}`
        } ${grab ? 'cursor-grab active:cursor-grabbing' : 'cursor-pointer'} ${className}`}
      {...rest}
    >
      {used !== undefined && <UsedDot used={used} />}
      {children}
    </button>
  )
}

// faint text on the right of a row: a length, a date, a folder
export const RowHint = ({ children, className = '' }: React.PropsWithChildren<{ className?: string }>) => (
  <span className={`shrink-0 text-[10px] text-slate-400 truncate ${className}`}>{children}</span>
)

export default BrowserRow
