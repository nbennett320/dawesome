import React from 'react'
import { dbToPosition, formatDb, MAX_DB, MIN_DB, positionToDb, SCALE_MARKS } from '../../types/mixer'

interface Props {
  label: string
  // decibels, MIN_DB (silent) to MAX_DB
  value: number
  onChange: (db: number) => void
  // double clicking calls this
  onReset: () => void
  // drawn beside the fader, e.g. a meter
  children?: React.ReactNode
}

export const THUMB_HEIGHT = 14

// keyboard steps, in dB
const STEP = 0.5
const FINE_STEP = 0.1
const PAGE_STEP = 3

const clampDb = (db: number) => Math.min(MAX_DB, Math.max(MIN_DB, db))

// a vertical volume fader. drag the handle (hold shift for fine moves), click
// the track to jump there, use the arrow keys, or double-click to reset
const Fader = ({ label, value, onChange, onReset, children }: Props) => {
  const trackRef = React.useRef<HTMLDivElement>(null)
  // where a drag started: the pointer's y and the fader's position
  const drag = React.useRef<{ y: number, position: number } | null>(null)

  // the height the handle can travel
  const travel = () => Math.max(1, (trackRef.current?.clientHeight ?? 1) - THUMB_HEIGHT)

  // the fader position under a pointer
  const positionAt = (clientY: number) => {
    const rect = trackRef.current?.getBoundingClientRect()
    if(!rect) return dbToPosition(value)
    return 1 - (clientY - rect.top - THUMB_HEIGHT / 2) / travel()
  }

  const setPosition = (position: number) => {
    onChange(clampDb(positionToDb(Math.min(1, Math.max(0, position)))))
  }

  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if(e.button !== 0) return
    e.currentTarget.setPointerCapture(e.pointerId)
    e.currentTarget.focus()

    // grabbing the handle moves it from where it is. clicking elsewhere on
    // the track jumps there first
    const onThumb = (e.target as HTMLElement).dataset.thumb !== undefined
    const position = onThumb ? dbToPosition(value) : positionAt(e.clientY)
    if(!onThumb) setPosition(position)

    drag.current = { y: e.clientY, position }
  }

  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if(!drag.current) return

    const scale = e.shiftKey ? 0.25 : 1
    const moved = (drag.current.y - e.clientY) / travel() * scale
    setPosition(drag.current.position + moved)

    // fine moves are relative to here, so switching shift doesn't jump
    if(e.shiftKey) drag.current = { y: e.clientY, position: drag.current.position + moved }
  }

  const endDrag = () => { drag.current = null }

  const onKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
    // from silent, the first step up goes to the bottom of the scale
    const from = value <= MIN_DB ? MIN_DB : value
    const step = e.shiftKey ? FINE_STEP : STEP

    const next = ({
      ArrowUp: from + step,
      ArrowRight: from + step,
      ArrowDown: from - step,
      ArrowLeft: from - step,
      PageUp: from + PAGE_STEP,
      PageDown: from - PAGE_STEP,
      Home: MAX_DB,
      End: MIN_DB,
    } as Record<string, number>)[e.key]

    if(next === undefined) return
    e.preventDefault()
    onChange(clampDb(Math.round(next * 10) / 10))
  }

  const position = dbToPosition(value)

  return (
    <div className='flex flex-row h-full gap-1'>
      {/* scale */}
      <div className='relative w-5 text-[9px] leading-none text-gray-400 select-none' aria-hidden>
        {SCALE_MARKS.map(db => (
          <span
            key={db}
            className='absolute right-0'
            style={{ bottom: `calc(${dbToPosition(db)} * (100% - ${THUMB_HEIGHT}px) + ${THUMB_HEIGHT / 2 - 4}px)` }}
          >
            {db > 0 ? `+${db}` : db}
          </span>
        ))}
      </div>

      <div
        ref={trackRef}
        role='slider'
        tabIndex={0}
        aria-label={label}
        aria-orientation='vertical'
        aria-valuemin={MIN_DB}
        aria-valuemax={MAX_DB}
        aria-valuenow={value}
        aria-valuetext={`${formatDb(value)} dB`}
        title='Drag (shift for fine moves), or double-click to reset'
        className='relative w-6 h-full cursor-ns-resize touch-none outline-none rounded focus-visible:ring-2 focus-visible:ring-sky-400'
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endDrag}
        onPointerCancel={endDrag}
        onDoubleClick={onReset}
        onKeyDown={onKeyDown}
      >
        {/* groove */}
        <div
          className='absolute left-1/2 -translate-x-1/2 w-1 rounded bg-gray-300'
          style={{ top: THUMB_HEIGHT / 2, bottom: THUMB_HEIGHT / 2 }}
        />
        {/* 0 dB mark */}
        <div
          className='absolute left-0 right-0 h-px bg-gray-400'
          style={{ bottom: `calc(${dbToPosition(0)} * (100% - ${THUMB_HEIGHT}px) + ${THUMB_HEIGHT / 2}px)` }}
        />
        {/* handle */}
        <div
          data-thumb
          className='absolute left-0 right-0 rounded-sm border border-gray-500 bg-gradient-to-b from-gray-100 to-gray-300 shadow'
          style={{
            height: THUMB_HEIGHT,
            bottom: `calc(${position} * (100% - ${THUMB_HEIGHT}px))`,
          }}
        />
      </div>

      {children}
    </div>
  )
}

export default Fader
