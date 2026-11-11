import React from 'react'
import { Jack, Led } from '../../components/fourtrack'
import { KeysIcon } from '../../components/toolbars/browser/icons'

interface Props {
  // the strip's name, for screen readers
  channel: string
  // how many effects the channel has
  count: number
  // at least one of them is on
  lit: boolean
  // the track has an instrument (counted with the effects)
  instrument?: boolean
  onOpen: () => void
}

const DROP_HINT = 'drag an effect from the sidebar onto the strip to add one'

// what the jack says it opens
export const fxTitle = (count: number, lit: boolean, instrument: boolean): string => {
  if(!count) return `Open the Effects Rack, or ${DROP_HINT}`
  const effects = count - (instrument ? 1 : 0)
  const what = [
    instrument ? 'An instrument' : '',
    effects ? `${effects} effect${effects === 1 ? '' : 's'}` : '',
  ].filter(Boolean).join(' and ')
  return `${what}${lit ? '' : ', all off'}. Open ${count === 1 ? 'it' : 'them'} in the Effects Rack, or ${DROP_HINT}`
}

// the foot of a strip: a quarter-inch jack, as if patched to an effects
// rack. empty without effects; with some, a plug in it and how many, and a
// light that's lit while any of them is on. opens the Effects Rack on the
// channel
const FxJack = ({ channel, count, lit, instrument = false, onOpen }: Props) => (
  <button
    type='button'
    className='ft-tray w-full flex flex-row items-center gap-1.5 rounded-md px-1.5 py-1 text-ink-soft hover:text-ink hover:ring-1 hover:ring-line-strong outline-none focus-visible:ring-2 focus-visible:ring-accent-focus'
    aria-label={`${channel} effects`}
    title={fxTitle(count, lit, instrument)}
    onClick={onOpen}
  >
    <Led color='blue' state={lit ? 'on' : 'off'} />
    <Jack plugged={count > 0} />
    <span className='flex flex-col items-start leading-none gap-1'>
      <span className='ft-label'>FX</span>
      {count > 0 && (
        <span className='inline-flex items-center gap-0.5 text-xs font-semibold tabular-nums' data-testid='fx-count'>
          {instrument && <KeysIcon className='w-3 h-3' />}
          {count}
        </span>
      )}
    </span>
  </button>
)

export default FxJack
