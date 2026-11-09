import React from 'react'

// sets kept while the app's open, by name: a browser's open folders stay
// open when the sidebar shows another page and comes back
const sets = new Map<string, Set<string>>()

type Update = Set<string> | ((prev: Set<string>) => Set<string>)

// like useState, for a set kept by name. updates can be a function of the
// set as it is, so several in a row each see the last
export const useSessionSet = (name: string): [Set<string>, (next: Update) => void] => {
  const [value, setValue] = React.useState(() => sets.get(name) ?? new Set<string>())

  const update = React.useCallback((next: Update) => {
    const prev = sets.get(name) ?? new Set<string>()
    const resolved = typeof next === 'function' ? next(prev) : next
    sets.set(name, resolved)
    setValue(resolved)
  }, [name])

  return [value, update]
}
