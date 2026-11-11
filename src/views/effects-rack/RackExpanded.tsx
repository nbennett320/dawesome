import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import {
  dockEffect, expandSlot, moveDockedEffect, openEffectWindow, undockEffect,
} from '../../state/slices/effectsSlice'
import { ChannelId, dockable, EditorInfo, EffectSlot, Rect } from '../../types/effects'
import { Notice } from '../../components/common'
import { AnalogButton } from '../../components/fourtrack'
import ChannelSelect from './ChannelSelect'
import { CollapseIcon, PopOutIcon, StepIcon } from './icons'
import { OnKey, RackStrip, SlotName, SmallKey } from './RackStack'

// where the docked interface goes: the part of the page it may cover (the
// scroll area, without its scrollbars) and its top-left corner (the
// placeholder's), in CSS pixels from the webview's top-left
export interface Placement {
  clip: Rect
  x: number
  y: number
}

export const measurePlacement = (area: HTMLElement, box: HTMLElement): Placement => {
  const a = area.getBoundingClientRect()
  const b = box.getBoundingClientRect()

  return {
    clip: {
      x: Math.round(a.left + area.clientLeft),
      y: Math.round(a.top + area.clientTop),
      width: Math.round(area.clientWidth),
      height: Math.round(area.clientHeight),
    },
    x: Math.round(b.left),
    y: Math.round(b.top),
  }
}

const samePlacement = (a: Placement | null, b: Placement) =>
  !!a && a.x === b.x && a.y === b.y && a.clip.x === b.clip.x && a.clip.y === b.clip.y
  && a.clip.width === b.clip.width && a.clip.height === b.clip.height

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// the plugin's own interface, docked: the engine lays its native view over
// the placeholder here, which is the interface's size and centred when
// there's room. the native view sits above the page, so nothing of the
// page may go over the placeholder: every control stays in the strip above
//
// it's docked while `active` (the tab is showing), and undocked when that
// stops, the slot changes, or this goes away (collapsed, removed, another
// channel)
const DockedEditor = ({ slot, active }: { slot: EffectSlot, active: boolean }) => {
  const dispatch = useAppDispatch()
  const areaRef = React.useRef<HTMLDivElement>(null)
  const boxRef = React.useRef<HTMLDivElement>(null)
  const [info, setInfo] = React.useState<EditorInfo | null>(null)
  const [error, setError] = React.useState<string | null>(null)
  // where it was last put, so the engine is only told when that changes
  const placed = React.useRef<Placement | null>(null)

  React.useEffect(() => {
    const area = areaRef.current
    const box = boxRef.current
    if(!active || !area || !box) return undefined

    let live = true
    setInfo(null)
    setError(null)
    const placement = measurePlacement(area, box)
    placed.current = placement

    dispatch(dockEffect(slot.id, placement.clip, placement.x, placement.y))
      .then(editor => { if(live) setInfo(editor) })
      .catch(e => { if(live) setError(message(e)) })

    return () => {
      live = false
      placed.current = null
      dispatch(undockEffect()).catch(() => {})
    }
  }, [dispatch, active, slot.id])

  // follow the page: the pane resized or moved, the area scrolled, the
  // window resized, or the placeholder took the interface's size. at most
  // once a frame
  React.useEffect(() => {
    const area = areaRef.current
    if(!active || !info || !area) return undefined

    let frame = 0
    const send = () => {
      frame = 0
      const box = boxRef.current
      if(!box) return
      const placement = measurePlacement(area, box)
      if(samePlacement(placed.current, placement)) return
      placed.current = placement
      dispatch(moveDockedEffect(placement.clip, placement.x, placement.y)).catch(() => {})
    }
    const schedule = () => { if(!frame) frame = requestAnimationFrame(send) }

    // (the placeholder has just taken the interface's size)
    schedule()

    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(schedule)
    observer?.observe(area)
    if(boxRef.current) observer?.observe(boxRef.current)
    // panes beside this one, the sidebar, a notice above the workspace
    observer?.observe(document.body)
    area.addEventListener('scroll', schedule, { passive: true })
    window.addEventListener('resize', schedule)

    return () => {
      if(frame) cancelAnimationFrame(frame)
      observer?.disconnect()
      area.removeEventListener('scroll', schedule)
      window.removeEventListener('resize', schedule)
    }
  }, [dispatch, active, info])

  return (
    <div ref={areaRef} className='flex-1 min-h-0 overflow-auto' data-docked-effect={slot.id}>
      {/* as wide and tall as the area, or the interface when it's bigger */}
      <div className='flex min-w-full min-h-full w-max p-2'>
        {error ? (
          <div className='m-auto max-w-md flex flex-col items-start gap-2'>
            <Notice tone='danger'>
              <p className='text-xs'>Couldn’t show {slot.name} here: {error}</p>
            </Notice>
            <AnalogButton
              variant='text'
              size='sm'
              onPress={() => { dispatch(openEffectWindow(slot.id)).catch(() => {}) }}
            >
              Open in a window
            </AnalogButton>
          </div>
        ) : (
          <div
            ref={boxRef}
            className='m-auto flex-none rounded-sm bg-sunken'
            style={{ width: info?.width ?? 0, height: info?.height ?? 0 }}
            data-testid='docked-placeholder'
          />
        )}
      </div>
    </div>
  )
}

interface Props {
  channel: ChannelId
  // the channel's chain
  slots: EffectSlot[]
  // the one expanded
  slot: EffectSlot
  // the tab is showing
  active: boolean
}

// one effect, its own interface docked in the tab, under a strip with the
// channel, its On key, stepping through the channel's other effects,
// collapsing back to the stack, and popping it out into a window
const RackExpanded = ({ channel, slots, slot, active }: Props) => {
  const dispatch = useAppDispatch()
  const steps = dockable(slots)
  const at = steps.findIndex(s => s.id === slot.id)
  const prev = steps[at - 1]
  const next = steps[at + 1]

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <RackStrip>
        <ChannelSelect channel={channel} />
        <OnKey slot={slot} />
        <div className='flex-1 min-w-0 flex flex-row items-center gap-2'>
          <SlotName slot={slot} />
        </div>

        <div className='flex flex-row items-center gap-1 shrink-0'>
          <SmallKey
            aria-label='Previous effect'
            title={prev ? `Show ${prev.name}` : undefined}
            disabled={!prev}
            onPress={() => { if(prev) dispatch(expandSlot(prev.id)) }}
          >
            <StepIcon direction='prev' />
          </SmallKey>
          <span className='ft-label tabular-nums tracking-normal' aria-label='Effect shown'>
            {at + 1} / {steps.length}
          </span>
          <SmallKey
            aria-label='Next effect'
            title={next ? `Show ${next.name}` : undefined}
            disabled={!next}
            onPress={() => { if(next) dispatch(expandSlot(next.id)) }}
          >
            <StepIcon direction='next' />
          </SmallKey>
        </div>

        <SmallKey
          aria-label='Back to the list'
          title='Back to every effect on the channel'
          onPress={() => dispatch(expandSlot(null))}
        >
          <CollapseIcon />
        </SmallKey>
        <div className='w-2 shrink-0' />
        <SmallKey
          aria-label={`Open ${slot.name} in a window`}
          title='Open its interface in a window'
          onPress={() => { dispatch(openEffectWindow(slot.id)).catch(() => {}) }}
        >
          <PopOutIcon />
        </SmallKey>
      </RackStrip>

      {/* afresh for each effect, and each time the tab is shown */}
      <DockedEditor key={`${slot.id}:${active}`} slot={slot} active={active} />
    </div>
  )
}

export default RackExpanded
