import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { paramEdited, selectParamsChanged } from '../../state/slices/effectsSlice'
import { currentGesture } from '../../state/gesture'
import { trackParamSend } from '../../state/effectParamSends'
import { EffectParams, ParamValue, ParamWire, plainOf } from '../../types/nativeEffect'

export type ParamsStatus =
  | { kind: 'loading' }
  | { kind: 'error', message: string }
  | { kind: 'ready', effect: string }

export interface EffectParamsApi {
  status: ParamsStatus
  // in id order (empty until they've come)
  params: ParamWire[]
  // set one by its normalized value: shown at once, and corrected by what
  // the engine says it became
  set: (id: number, normalized: number) => void
  // set one from what was typed ("2.5k", "-3 dB"). what can't be read
  // changes nothing
  setText: (id: number, text: string) => void
}

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// a built-in effect's parameters, kept here while its panel is showing. the
// engine has the real values: each set hands back what the parameter
// became. a knob dragged fast sends many sets, which may come back out of
// order: only the answer to the latest set of each parameter is kept
//
// each set is undoable: those sent while the mouse is held (a drag) carry
// its gesture, and are undone as one. after an undo (or a project opening)
// they're fetched again
export const useEffectParams = (slotId: number): EffectParamsApi => {
  const dispatch = useAppDispatch()
  const changed = useAppSelector(selectParamsChanged)
  const [status, setStatus] = React.useState<ParamsStatus>({ kind: 'loading' })
  const [params, setParams] = React.useState<ParamWire[]>([])
  // each parameter's latest set, by id
  const latest = React.useRef(new Map<number, number>())
  // the slot last fetched: another one starts from "loading", the same one
  // fetched again keeps showing what it had
  const fetched = React.useRef<number | null>(null)

  React.useEffect(() => {
    let live = true
    if(fetched.current !== slotId) {
      setStatus({ kind: 'loading' })
      latest.current = new Map()
    }
    fetched.current = slotId
    invoke<EffectParams>('get_effect_params', { slotId })
      .then(got => {
        if(!live) return
        setParams(got.params)
        setStatus({ kind: 'ready', effect: got.effect })
      })
      .catch(e => {
        if(live) setStatus({ kind: 'error', message: message(e) })
      })
    return () => { live = false }
  }, [slotId, changed])

  const apply = React.useCallback((value: ParamValue) => {
    setParams(prev => prev.map(p => (p.id === value.id ? { ...p, value } : p)))
  }, [])

  // send a set, keeping its answer only if no later set of the same
  // parameter went after it
  const send = React.useCallback((id: number, request: () => Promise<ParamValue>) => {
    const seq = (latest.current.get(id) ?? 0) + 1
    latest.current.set(id, seq)
    trackParamSend(request())
      .then(value => {
        if(latest.current.get(id) === seq) apply(value)
        dispatch(paramEdited())
      })
      .catch(logError('set the effect’s parameter'))
  }, [apply, dispatch])

  const set = React.useCallback((id: number, normalized: number) => {
    // where it's going, before the engine says (its text follows the answer)
    setParams(prev => prev.map(p => (p.id === id
      ? { ...p, value: { ...p.value, normalized, value: plainOf(p, normalized) } }
      : p)))
    const gesture = currentGesture()
    send(id, () => invoke<ParamValue>('set_effect_param', { slotId, id, normalized, gesture }))
  }, [slotId, send])

  const setText = React.useCallback((id: number, text: string) => {
    send(id, () => invoke<ParamValue>('set_effect_param_text', { slotId, id, text }))
  }, [slotId, send])

  return { status, params, set, setText }
}
