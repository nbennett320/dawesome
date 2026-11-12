import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { beginSampleDrag, onSampleDrag } from '../../state/sampleDrag'
import {
  highlightSlot,
  openPlugin,
  removeEffect,
  selectHighlightSlotId,
  setEffectBypassed,
} from '../../state/slices/effectsSlice'
import { ChannelId, EffectSlot, hasTab } from '../../types/effects'
import { AnalogButton, Led } from '../../components/fourtrack'
import ChannelSelect from './ChannelSelect'
import { RACK_ROW, useRackDrop } from './useRackDrop'
import { CloseIcon, GripIcon, PopOutIcon, TabIcon } from '../../components/icons'

// how long a slot just dropped in stays pointed out
const HIGHLIGHT_MS = 1500

// a small key, the size of the tab bar's
export const SmallKey = ({ children, ...props }: React.ComponentProps<typeof AnalogButton>) => (
  <AnalogButton size='xs' className='ft-btn--tab' {...props}>{children}</AnalogButton>
)

// switching an effect on or off: a light, and a key held down while it's on
export const OnKey = ({ slot }: { slot: EffectSlot }) => {
  const dispatch = useAppDispatch()
  const on = !slot.bypassed

  return (
    <div className='flex flex-row items-center gap-1 shrink-0'>
      <Led color='blue' state={on && !slot.missing ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='xs'
        latched={on}
        disabled={slot.missing}
        aria-label={`${slot.name} on`}
        title={on ? 'On. Click to bypass it' : 'Bypassed. Click to switch it on'}
        onPress={() => { dispatch(setEffectBypassed(slot.id, on)).catch(() => {}) }}
      >
        On
      </AnalogButton>
    </div>
  )
}

// an effect's name, with its maker dimmed (or that it's not installed)
const SlotName = ({ slot }: { slot: EffectSlot }) => (
  <div className='flex-1 min-w-0 flex flex-row items-baseline gap-1.5' title={`${slot.name} by ${slot.vendor}`}>
    <span className={`truncate text-xs font-medium ${slot.missing ? 'text-ink-faint italic' : 'text-ink-strong'}`}>
      {slot.name}
    </span>
    <span className='truncate text-[10px] text-ink-faint shrink-[2]'>
      {slot.missing ? 'Not installed' : slot.vendor}
    </span>
  </div>
)

interface RowProps {
  slot: EffectSlot
  highlighted: boolean
  // being dragged elsewhere
  dragging: boolean
}

// one effect in the chain. drag it (by any part that isn't a key) to
// reorder it, or onto a mixer strip to move it to that channel
const RackRow = ({ slot, highlighted, dragging }: RowProps) => {
  const dispatch = useAppDispatch()
  // where Preferences say a plugin's interface opens
  const inTab = useAppSelector(state => state.appearance.preferences.pluginView === 'tab')
  const ref = React.useRef<HTMLDivElement>(null)

  React.useEffect(() => {
    if(highlighted) ref.current?.scrollIntoView?.({ block: 'nearest' })
  }, [highlighted])

  let tone = ''
  if(highlighted) tone = 'ring-2 ring-inset ring-accent-focus bg-accent-tint'
  else if(slot.missing) tone = 'opacity-60'

  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-element-interactions -- dragging is by mouse only, its keys work from the keyboard
    <div
      ref={ref}
      role='listitem'
      aria-label={slot.name}
      className={`ft-strip flex flex-row items-center gap-2 h-9 pl-1.5 pr-2 cursor-grab ${tone} ${dragging ? 'opacity-40' : ''}`}
      {...{ [RACK_ROW]: slot.id }}
      onMouseDown={ev => {
        if(ev.button !== 0) return
        if((ev.target as HTMLElement).closest('button, select, input')) return
        beginSampleDrag({ kind: 'slot', slotId: slot.id, label: slot.name }, ev)
      }}
    >
      <span className='text-ink-faint' title='Drag to reorder, or onto a mixer channel to move it there'>
        <GripIcon />
      </span>
      <OnKey slot={slot} />
      <SlotName slot={slot} />

      {hasTab(slot) && (
        <SmallKey
          aria-label={`Open ${slot.name}`}
          title={`Open its interface in a ${inTab ? 'tab' : 'window'} of its own (Preferences choose which)`}
          onPress={() => { dispatch(openPlugin(slot.id)).catch(() => {}) }}
        >
          {inTab ? <TabIcon /> : <PopOutIcon />}
        </SmallKey>
      )}
      <SmallKey
        aria-label={`Remove ${slot.name}`}
        title='Take it off the channel'
        onPress={() => { dispatch(removeEffect(slot.id)).catch(() => {}) }}
      >
        <CloseIcon />
      </SmallKey>
    </div>
  )
}

// where a dragged effect would go
const DropLine = () => (
  <div className='relative h-0' aria-hidden>
    <div className='absolute inset-x-1 -top-[3px] h-0.5 rounded bg-accent' />
  </div>
)

// the strip along the top of the tab
const RackStrip = ({ children }: { children: React.ReactNode }) => (
  <div className='shrink-0 px-2 py-1'>
    <div className='ft-tray rounded-md flex flex-row items-center gap-2 px-2 py-1.5 min-h-[34px]'>
      {children}
    </div>
  </div>
)

// a channel's effects as a list, in the order the sound goes through them
const RackStack = ({ channel, slots }: { channel: ChannelId, slots: EffectSlot[] }) => {
  const dispatch = useAppDispatch()
  const highlighted = useAppSelector(selectHighlightSlotId)
  const listRef = React.useRef<HTMLDivElement>(null)
  const gap = useRackDrop(listRef, channel)
  const [dragged, setDragged] = React.useState<number | null>(null)

  // the pointed-out slot goes back to normal after a moment
  React.useEffect(() => {
    if(highlighted === null) return undefined
    const timer = setTimeout(() => dispatch(highlightSlot(null)), HIGHLIGHT_MS)
    return () => clearTimeout(timer)
  }, [dispatch, highlighted])

  // the row being dragged is dimmed until it's dropped
  React.useEffect(() => onSampleDrag(({ type, item }) => {
    setDragged(type === 'move' && item.kind === 'slot' ? item.slotId : null)
  }), [])

  const out = channel === 'master' ? 'Master fader' : 'Fader'
  const count = slots.length ? `${slots.length} effect${slots.length === 1 ? '' : 's'}` : 'No effects'

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <RackStrip>
        <ChannelSelect channel={channel} />
        <span className='ft-label normal-case tracking-normal'>
          {count}
        </span>
      </RackStrip>

      <div
        ref={listRef}
        className='flex-1 min-h-0 overflow-y-auto px-3 pb-3'
      >
        <div className='max-w-xl flex flex-col'>
          <div className='ft-label text-center py-1'>In ↓</div>

          {slots.length ? (
            <div role='list' aria-label='Effects, in order' className='flex flex-col gap-1.5'>
              {slots.map((slot, i) => (
                <React.Fragment key={slot.id}>
                  {gap === i && <DropLine />}
                  <RackRow slot={slot} highlighted={highlighted === slot.id} dragging={dragged === slot.id} />
                </React.Fragment>
              ))}
              {gap === slots.length && <DropLine />}
            </div>
          ) : (
            <div
              className={`rounded border border-dashed px-3 py-6 text-center text-xs ${
                gap !== null ? 'border-accent-focus bg-accent-tint text-ink' : 'border-line-strong text-ink-faint'}`}
            >
              Drag an effect here
            </div>
          )}

          <div className='ft-label text-center py-1'>↓ {out}</div>
        </div>
      </div>
    </div>
  )
}

export default RackStack
