import React from 'react'
import { useAppDispatch } from '../../hooks/redux'
import {
  dockEffect, moveDockedEffect, openEffectWindow, resizeDockedEffect, Size, undockEffect,
} from '../../state/slices/effectsSlice'
import { EditorInfo, EffectSlot, Rect } from '../../types/effects'
import { Notice } from '../../components/common'
import { AnalogButton } from '../../components/fourtrack'

// the space kept around the interface (p-2)
const PAD = 8

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

// the room a resizable interface may fill: the area less its padding. its
// whole size, scrollbars or not, so scrollbars coming and going don't
// resize it
export const measureFill = (area: HTMLElement): Size => ({
  width: Math.max(0, Math.round(area.offsetWidth - 2 * PAD)),
  height: Math.max(0, Math.round(area.offsetHeight - 2 * PAD)),
})

const samePlacement = (a: Placement | null, b: Placement) =>
  !!a && a.x === b.x && a.y === b.y && a.clip.x === b.clip.x && a.clip.y === b.clip.y
  && a.clip.width === b.clip.width && a.clip.height === b.clip.height

const sameSize = (a: Size | null, b: Size) => !!a && a.width === b.width && a.height === b.height

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// a plugin's own interface, docked: the engine lays its native view over
// the placeholder here, which takes the size the engine says it is, centerd
// on the panel. the native view sits above the page, so nothing of the page
// may go over the placeholder
//
// it's docked while `active` (its tab is showing, and no tab is being
// dragged), and undocked when that stops or this goes away
const DockedEditor = ({ slot, active }: { slot: Pick<EffectSlot, 'id' | 'name'>, active: boolean }) => {
  const dispatch = useAppDispatch()
  const areaRef = React.useRef<HTMLDivElement>(null)
  const boxRef = React.useRef<HTMLDivElement>(null)
  const [info, setInfo] = React.useState<EditorInfo | null>(null)
  const [error, setError] = React.useState<string | null>(null)
  // where it was last put, and the room it was last given, so the engine is
  // only told when they change
  const placed = React.useRef<Placement | null>(null)
  const filled = React.useRef<Size | null>(null)

  React.useEffect(() => {
    const area = areaRef.current
    const box = boxRef.current
    if(!active || !area || !box) return undefined

    let live = true
    setInfo(null)
    setError(null)
    const placement = measurePlacement(area, box)
    const fill = measureFill(area)
    placed.current = placement
    filled.current = fill

    dispatch(dockEffect(slot.id, placement.clip, placement.x, placement.y, fill))
      .then(editor => { if(live) setInfo(editor) })
      .catch(e => { if(live) setError(message(e)) })

    return () => {
      live = false
      placed.current = null
      filled.current = null
      dispatch(undockEffect(slot.id)).catch(() => {})
    }
  }, [dispatch, active, slot.id])

  const docked = info !== null
  const resizable = !!info?.resizable

  // follow the page: the pane resized or moved, the area scrolled, the
  // window resized, or the placeholder took the interface's size. a
  // resizable interface is given the new room. at most once a frame
  React.useEffect(() => {
    const area = areaRef.current
    if(!active || !docked || !area) return undefined

    let live = true
    let frame = 0
    const send = () => {
      frame = 0
      const box = boxRef.current
      if(!box) return

      const fill = measureFill(area)
      if(resizable && !sameSize(filled.current, fill)) {
        filled.current = fill
        dispatch(resizeDockedEffect(slot.id, fill))
          .then(editor => { if(live) setInfo(editor) })
          .catch(() => {})
      }

      const placement = measurePlacement(area, box)
      if(samePlacement(placed.current, placement)) return
      placed.current = placement
      dispatch(moveDockedEffect(slot.id, placement.clip, placement.x, placement.y)).catch(() => {})
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
      live = false
      if(frame) cancelAnimationFrame(frame)
      observer?.disconnect()
      area.removeEventListener('scroll', schedule)
      window.removeEventListener('resize', schedule)
    }
  }, [dispatch, active, docked, resizable, slot.id])

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
          // no fill of its own: the panel shows until the native view's there
          <div
            ref={boxRef}
            className='m-auto flex-none'
            style={{ width: info?.width ?? 0, height: info?.height ?? 0 }}
            data-testid='docked-placeholder'
          />
        )}
      </div>
    </div>
  )
}

export default DockedEditor
