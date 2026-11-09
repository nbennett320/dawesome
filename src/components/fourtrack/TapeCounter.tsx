import React from 'react'

export interface TapeCounterProps {
  value: number
  digits?: number
  label?: string
}

// mechanical counter wheels
export const TapeCounter = ({ value, digits = 3, label = 'Counter' }: TapeCounterProps) => {
  const mod = 10 ** digits
  const text = String(((Math.floor(value) % mod) + mod) % mod).padStart(digits, '0')
  return (
    <div className='ft-counter' role='img' aria-label={`${label} ${text}`}>
      {text.split('').map((d, i) => (
        // the wheels are fixed in place, so their position is their identity
        // eslint-disable-next-line react/no-array-index-key
        <span key={i} className='ft-counter__digit'>{d}</span>
      ))}
    </div>
  )
}
