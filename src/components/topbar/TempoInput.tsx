import React from 'react'
import { useStore } from 'react-redux'
import type { RootState } from '../../state/store'
import { selectPlaylistTempo, validTempo } from '../../state/slices/playlistSlice'
import { changeTempo } from '../../state/slices/sequencerSlice'
import { selectRecording } from '../../state/slices/recordingSlice'
import { currentGesture, newGesture } from '../../state/gesture'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'
import { RollingCounter } from '../fourtrack'
import { clamp, cx } from '../../util/helpers'

export const MIN_TEMPO = 1
export const MAX_TEMPO = 1000
// pixels of vertical drag for one beat per minute, and with shift held
export const PX_PER_BPM = 4
export const FINE_PX_PER_BPM = 16

// the tempo on counter wheels: whole tempos as three digits (090), others
// with a tenth (128.5)
export const tempoWheels = (tempo: number): string => {
  const tenths = Math.round(tempo * 10)
  const whole = String(Math.floor(tenths / 10)).padStart(3, '0')
  return tenths % 10 === 0 ? whole : `${whole}.${tenths % 10}`
}

// the tempo a drag has reached: where it's got to (a fraction, so slow
// drags still add up), in whole beats per minute, kept to what the engine
// takes
export const draggedTempo = (at: number): number => clamp(Math.round(at), MIN_TEMPO, MAX_TEMPO)

// the tempo on rolling counter wheels. drag up for faster, down for slower
// (shift for fine moves), arrow keys (shift for steps of ten), or
// double-click (or Enter) to type one, taken up on Enter or when the field
// is left. a drag is one step to undo
const TempoInput = () => {
  const tempo = useAppSelector(selectPlaylistTempo)
  // the tempo can't change under a take
  const recording = useAppSelector(selectRecording)
  const dispatch = useAppDispatch()
  const store = useStore<RootState>()
  const [typing, setTyping] = React.useState(false)
  // (the field is left once: Enter or Escape, then the blur of it going)
  const left = React.useRef(false)
  // the tempo being dragged (or stepped) to, shown until the engine has it
  const [dragged, setDragged] = React.useState<number | null>(null)
  // a drag in progress: where the mouse was last, where the tempo has got to
  // (unrounded), and the gesture its changes are undone as
  const drag = React.useRef<{ y: number, at: number, gesture: number | null } | null>(null)
  // tempos are sent one at a time, so they reach the engine in order. while
  // one is on its way, only the latest one asked for waits behind it
  const sending = React.useRef(false)
  const queued = React.useRef<{ to: number, gesture: number | null } | null>(null)
  const dial = React.useRef<HTMLDivElement>(null)

  const showSent = () => {
    if(!drag.current && !sending.current) setDragged(null)
  }

  const send = async (to: number, gesture: number | null) => {
    queued.current = { to, gesture }
    if(sending.current) return
    sending.current = true
    while(queued.current) {
      const next = queued.current
      queued.current = null
      // (the tempo as it is now, after any sent before this one)
      if(next.to !== selectPlaylistTempo(store.getState())) {
        // eslint-disable-next-line no-await-in-loop
        await dispatch(changeTempo(next.to, next.gesture))
      }
    }
    sending.current = false
    showSent()
  }

  // a tempo refused (recording started) goes back to the one it is
  const set = (to: number, gesture: number | null) => {
    if(!validTempo(to)) return
    setDragged(to)
    void send(to, gesture)
  }

  // - refocus: back on the wheels, for the arrow keys (on Enter or Escape;
  //   not when the field was left by clicking somewhere else)
  const leave = (typed: string | null, refocus: boolean) => {
    if(left.current) return
    left.current = true
    if(typed !== null && typed.trim() !== '') set(parseFloat(typed), newGesture())
    setTyping(false)
    if(refocus) setTimeout(() => dial.current?.focus(), 0)
  }

  const startTyping = () => {
    if(recording) return
    left.current = false
    setTyping(true)
  }

  // a recording started: put down what was being typed or dragged
  React.useEffect(() => {
    if(!recording) return
    setTyping(false)
    drag.current = null
    showSent()
  }, [recording])

  const shown = dragged ?? tempo
  const title = recording
    ? 'The tempo can’t be changed while recording'
    : `Tempo in beats per minute, ${MIN_TEMPO} to ${MAX_TEMPO}. Drag up or down (Shift for fine), or double-click to type`

  const endDrag = () => {
    if(!drag.current) return
    drag.current = null
    showSent()
  }

  return (
    <div className='flex flex-row items-center gap-1.5'>
      <span className='ft-label'>Tempo</span>
      {typing ? (
        <input
          type='number'
          // eslint-disable-next-line jsx-a11y/no-autofocus
          autoFocus
          min={MIN_TEMPO}
          max={MAX_TEMPO}
          className='ft-readout text-sm py-0.5 w-16 h-7'
          aria-label='Playlist tempo input'
          defaultValue={tempo}
          onFocus={e => e.currentTarget.select()}
          onBlur={e => leave(e.currentTarget.value, false)}
          onKeyDown={e => {
            if(e.key === 'Enter') leave(e.currentTarget.value, true)
            if(e.key === 'Escape') leave(null, true)
          }}
        />
      ) : (
        <div
          ref={dial}
          role='spinbutton'
          tabIndex={recording ? -1 : 0}
          aria-label='Playlist tempo'
          aria-valuemin={MIN_TEMPO}
          aria-valuemax={MAX_TEMPO}
          aria-valuenow={shown}
          aria-disabled={recording || undefined}
          title={title}
          className={cx(
            'rounded select-none touch-none focus-visible:outline focus-visible:outline-2 focus-visible:outline-accent-focus',
            recording ? 'cursor-not-allowed opacity-50' : 'cursor-ns-resize',
          )}
          onPointerDown={e => {
            if(recording || e.button !== 0) return
            e.currentTarget.setPointerCapture(e.pointerId)
            e.currentTarget.focus()
            // the press began a gesture (see state/gesture.ts): every tempo
            // the drag goes through is undone with it
            drag.current = { y: e.clientY, at: shown, gesture: currentGesture() }
          }}
          onPointerMove={e => {
            const d = drag.current
            if(!d) return
            d.at += (d.y - e.clientY) / (e.shiftKey ? FINE_PX_PER_BPM : PX_PER_BPM)
            d.at = clamp(d.at, MIN_TEMPO, MAX_TEMPO)
            d.y = e.clientY
            const to = draggedTempo(d.at)
            if(to !== shown) set(to, d.gesture)
          }}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onLostPointerCapture={endDrag}
          onDoubleClick={startTyping}
          onKeyDown={e => {
            if(recording) return
            if(e.key === 'Enter') {
              e.preventDefault()
              startTyping()
              return
            }
            const by = e.shiftKey ? 10 : 1
            const step = ({ ArrowUp: by, ArrowDown: -by, PageUp: 10, PageDown: -10 } as Record<string, number | undefined>)[e.key]
            if(step === undefined) return
            e.preventDefault()
            // from a tempo with a fraction, the next whole one that way
            const to = clamp(step > 0 ? Math.floor(shown) + step : Math.ceil(shown) + step, MIN_TEMPO, MAX_TEMPO)
            set(to, null)
          }}
        >
          <RollingCounter value={tempoWheels(shown)} label='Tempo' />
        </div>
      )}
    </div>
  )
}

export default TempoInput
