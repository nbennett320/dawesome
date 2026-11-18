import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { changeTapeMachine, selectTapeMachine } from '../../state/slices/tapeSlice'
import { selectRecording } from '../../state/slices/recordingSlice'
import { Knob } from '../../components/fourtrack'
import { formatPitch, PITCH, PITCH_DETENT } from '../../types/tape'

// the deck's PITCH knob, by its keys as on a Portastudio 414: the whole song
// faster and higher (+) or slower and lower (−), up to 12 %, clicking into
// place at 0 (T27 in context/tape-project). it's the tape's, so it only
// does anything with the tape on: off, it dims, but can still be set. it
// holds still while recording (a take is fitted to the speed it was
// recorded at)
const PitchKnob = () => {
  const dispatch = useAppDispatch()
  const machine = useAppSelector(selectTapeMachine)
  const recording = useAppSelector(selectRecording)
  const hint = machine.enabled
    ? 'Pitch: the whole song plays faster and higher, or slower and lower, as a tape deck\'s varispeed. '
      + 'Clicks into place at 0. Drag up or down (shift for fine moves), or double-click for 0'
    : 'Pitch: the tape\'s varispeed. It plays the song faster or slower with the Tape switch on'

  return (
    <div
      className={`flex flex-col items-center gap-1 transition-opacity ${machine.enabled ? '' : 'opacity-50'}`}
      data-hint={recording ? 'Pitch: holds still while recording' : hint}
    >
      <span className='ft-label text-[9px] tabular-nums tracking-normal normal-case'>{formatPitch(machine.pitch)}</span>
      <div className='flex flex-row items-center gap-0.5'>
        <span className='ft-label text-[10px] w-2 text-center' aria-hidden>−</span>
        <Knob
          label='Pitch'
          size={26}
          min={PITCH.min}
          max={PITCH.max}
          center={PITCH.default}
          detent={PITCH_DETENT}
          step={0.1}
          value={machine.pitch}
          onChange={pitch => dispatch(changeTapeMachine({ pitch }))}
          formatValue={formatPitch}
          disabled={recording}
        />
        <span className='ft-label text-[10px] w-2 text-center' aria-hidden>+</span>
      </div>
      <span className='ft-label text-[9px]'>Pitch</span>
    </div>
  )
}

export default PitchKnob
