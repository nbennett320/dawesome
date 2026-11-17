import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { changeTapeMachine, fetchTape, selectTapeMachine } from '../../state/slices/tapeSlice'
import { AnalogButton, Knob, Led, LedColor } from '../../components/fourtrack'
import {
  AGE, formatGainDb, formatPercent, HISS, MACHINES, OUTPUT_DB, WOBBLE,
} from '../../types/tape'

// the tape isn't built yet: the controls are kept with the project, but
// don't change the sound (as the Rec key says of recording)
// heard in Always live mode for now; print mode is direct until printing
// is built (Settings → Controls → Tape)
const NOT_YET = ' (heard with Settings → Controls → Tape → Always live, for now)'

const DRAG = 'Drag up or down (shift for fine moves)'

// a knob on the machine's panel: its value over it, its name under it, as
// the deck keys have their names under them
const MachineKnob = ({
  name, label, hint, value, range, step, format, onChange,
}: {
  name: string
  // what it's called to screen readers
  label: string
  hint: string
  value: number
  range: { min: number, max: number, default: number }
  step: number
  format: (value: number) => string
  onChange: (value: number) => void
}) => (
  <div className='flex flex-col items-center gap-1 min-w-12' data-hint={`${hint}. ${DRAG}, or double-click for ${format(range.default)}`}>
    <span className='ft-label text-[9px] tabular-nums tracking-normal normal-case'>{format(value)}</span>
    <Knob
      label={label}
      size={30}
      min={range.min}
      max={range.max}
      // what double-clicking or "0" goes back to
      center={range.default}
      step={step}
      value={value}
      onChange={onChange}
      formatValue={format}
    />
    <span className='ft-label text-[9px]'>{name}</span>
  </div>
)

// an on/off key with its light over it and its name under it
const MachineSwitch = ({
  name, label, hint, on, color, asWritten = false, onChange,
}: {
  name: string
  // keep the name's own case (dbx), rather than capitals
  asWritten?: boolean
  label: string
  hint: string
  on: boolean
  color: LedColor
  onChange: (on: boolean) => void
}) => (
  <div className='flex flex-col items-center gap-1'>
    <Led color={color} state={on ? 'on' : 'off'} />
    <AnalogButton
      variant='text'
      size='sm'
      latched={on}
      aria-label={label}
      data-hint={hint}
      onPress={() => onChange(!on)}
    >
      {on ? 'On' : 'Off'}
    </AnalogButton>
    <span className={`ft-label text-[9px] ${asWritten ? 'normal-case' : ''}`}>{name}</span>
  </div>
)

// the tape machine's controls, beside the deck keys (T22 in
// context/tape-project): the project's Tape switch, then the machine and
// its wear, dbx and output. with the tape off they dim, but can still be
// set ahead of turning it on
const TapeMachinePanel = () => {
  const dispatch = useAppDispatch()
  const machine = useAppSelector(selectTapeMachine)
  const change = (c: Parameters<typeof changeTapeMachine>[0]) => dispatch(changeTapeMachine(c))

  // the engine keeps the tape with the project: catch up with it when the
  // tab opens
  React.useEffect(() => {
    dispatch(fetchTape()).catch(e => {
      // eslint-disable-next-line no-console
      console.error('could not read the tape:', e)
    })
  }, [dispatch])

  return (
    <div className='ft-tray rounded-lg max-w-full flex flex-row items-end gap-2 px-3 pt-2 pb-1.5' role='group' aria-label='Tape machine'>
      <MachineSwitch
        name='Tape'
        label='Tape'
        color='amber'
        on={machine.enabled}
        hint={`Tape: on, the tracks put on tape (in the mixer) go through a cassette four-track before their channels. Off, the project plays without it${NOT_YET}`}
        onChange={enabled => change({ enabled })}
      />

      <div className='self-stretch w-px my-1 bg-line' aria-hidden />

      {/* wraps onto a second row when the tab is narrow */}
      <div className={`min-w-0 flex flex-row flex-wrap items-end justify-center gap-2 transition-opacity ${
        machine.enabled ? '' : 'opacity-50'}`}
      >
        <div className='flex flex-col items-center gap-1' data-hint='The deck the tape runs on'>
          <span className='h-2' aria-hidden />
          <select
            className='ft-readout text-[10px] py-0.5 text-left'
            aria-label='Tape machine'
            value={machine.machine}
            onChange={e => change({ machine: Number(e.currentTarget.value) })}
          >
            {MACHINES.map((name, index) => <option key={name} value={index}>{name}</option>)}
          </select>
          <span className='ft-label text-[9px]'>Machine</span>
        </div>
        <MachineKnob
          name='Age'
          label='Tape age'
          hint='Age: how worn the tape is. Fresh at 0 %, a worn cassette at 50 %, a crumbling 1950s reel at 100 %: more wobble, a sticky tape catching, more hiss and dropouts, duller highs'
          value={machine.age}
          range={AGE}
          step={1}
          format={formatPercent}
          onChange={age => change({ age })}
        />
        <MachineKnob
          name='W&F'
          label='Wow and flutter'
          hint="Wow & flutter: how much the tape's speed wavers. 100% is the real deck's, past it exaggerates"
          value={machine.wobble}
          range={WOBBLE}
          step={1}
          format={formatPercent}
          onChange={wobble => change({ wobble })}
        />
        <MachineKnob
          name='Hiss'
          label='Tape hiss'
          hint="Hiss: the tape's noise. 100% is the real deck's"
          value={machine.hiss}
          range={HISS}
          step={1}
          format={formatPercent}
          onChange={hiss => change({ hiss })}
        />
        <MachineSwitch
          name='dbx'
          asWritten
          label='dbx noise reduction'
          color='lime'
          on={machine.dbx}
          hint="dbx: the deck's noise reduction, which hushes the hiss (and pumps it a little under the music)"
          onChange={dbx => change({ dbx })}
        />
        <MachineKnob
          name='Output'
          label='Tape output'
          hint="Output: the tape's level after playback"
          value={machine.outputDb}
          range={OUTPUT_DB}
          step={0.1}
          format={formatGainDb}
          onChange={outputDb => change({ outputDb })}
        />
      </div>
    </div>
  )
}

export default TapeMachinePanel
