import React from 'react'
import { AnalogButton, Knob, Led } from '../../components/fourtrack'
import { ReadoutInput } from '../../components/common'
import { choiceNormalized, paramKind, ParamWire } from '../../types/nativeEffect'

export interface ParamControlProps {
  param: ParamWire
  // what it's called to screen readers, with its group ("Band 3 Freq")
  label: string
  set: (id: number, normalized: number) => void
  setText: (id: number, text: string) => void
}

const RESET = 'Double-click to reset'

// a continuous parameter: its name, a knob, and its value to type into.
// double-click the name or the knob to put it back to its default
const ParamKnob = ({ param, label, set, setText }: ParamControlProps) => {
  const reset = () => set(param.id, param.defaultNormalized)

  return (
    <div className='flex flex-col items-center gap-1 w-[4.5rem]'>
      <span className='ft-label truncate max-w-full' title={RESET} onDoubleClick={reset}>{param.name}</span>
      {/* double-clicking puts it exactly at its default (the knob's own
          double-click would round it to its step) */}
      <div
        onDoubleClickCapture={e => {
          e.stopPropagation()
          reset()
        }}
      >
        <Knob
          label={label}
          title='Drag up or down (shift for fine moves), or double-click to reset'
          size={40}
          min={0}
          max={1}
          step={0.001}
          center={param.defaultNormalized}
          value={param.value.normalized}
          onChange={n => set(param.id, n)}
          formatValue={() => param.value.text}
        />
      </div>
      <ReadoutInput
        aria-label={`${label} value`}
        className='w-full text-center text-[11px] py-0.5'
        value={param.value.text}
        onCommit={typed => { if(typed.trim() && typed !== param.value.text) setText(param.id, typed) }}
      />
    </div>
  )
}

// a switch: a key that stays down while it's on, and a light
const ParamSwitch = ({ param, label, set }: ParamControlProps) => {
  const on = param.value.value >= 0.5

  return (
    <div className='flex flex-col items-center gap-1 w-[4.5rem]'>
      <span className='ft-label truncate max-w-full'>{param.name}</span>
      <Led color='lime' state={on ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='xs'
        latched={on}
        aria-label={label}
        title={on ? 'On. Click to switch it off' : 'Off. Click to switch it on'}
        onPress={() => set(param.id, on ? 0 : 1)}
      >
        {param.value.text}
      </AnalogButton>
    </div>
  )
}

// one of a list
const ParamChoice = ({ param, label, set }: ParamControlProps) => (
  <div className='flex flex-col items-center gap-1 min-w-[4.5rem]'>
    <span className='ft-label truncate max-w-full'>{param.name}</span>
    <select
      aria-label={label}
      className='ft-readout text-[11px] py-0.5 text-left max-w-[8rem]'
      value={Math.round(param.value.value)}
      onChange={e => set(param.id, choiceNormalized(param, Number(e.currentTarget.value)))}
    >
      {param.labels.map((text, index) => <option key={text} value={index}>{text}</option>)}
    </select>
  </div>
)

// the control a parameter is drawn with, by what kind it is
const ParamControl = (props: ParamControlProps) => {
  switch(paramKind(props.param)) {
    case 'switch': return <ParamSwitch {...props} />
    case 'choice': return <ParamChoice {...props} />
    default: return <ParamKnob {...props} />
  }
}

export default ParamControl
