import React from 'react'

// helpers shared by the four-track controls

export const cx = (...c: Array<string | false | null | undefined>) => c.filter(Boolean).join(' ')

export const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v))

// a value that's either controlled (value given) or kept here
export function useControllable<T>(value: T | undefined, defaultValue: T, onChange?: (v: T) => void) {
  const [inner, setInner] = React.useState(defaultValue)
  const controlled = value !== undefined
  const current = controlled ? (value as T) : inner
  const set = React.useCallback(
    (v: T) => {
      if(!controlled) setInner(v)
      onChange?.(v)
    },
    [controlled, onChange],
  )
  return [current, set] as const
}
