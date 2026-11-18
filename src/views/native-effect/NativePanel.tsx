import React from 'react'
import { Notice } from '../../components/common'
import { groupParams } from '../../types/nativeEffect'
import ParamControl from './ParamControl'
import { EffectParamsApi, useEffectParams } from './useEffectParams'
import ChorusPanel from './chorus/ChorusPanel'
import DelayPanel from './delay/DelayPanel'
import EqPanel from './eq/EqPanel'
import FilterPanel from './filter/FilterPanel'
import FlangerPanel from './flanger/FlangerPanel'
import PhaserPanel from './phaser/PhaserPanel'

// a built-in effect's own panel, for one that wants more than knobs (the
// EQ's curve), by the effect's id. one that has none gets the generated
// panel
// - active: it can be seen (its tab is showing): live displays poll only
//   then
export type CustomPanel = React.ComponentType<{ api: EffectParamsApi, slotId: number, active: boolean }>
const PANELS: Record<string, CustomPanel> = {
  chorus: ChorusPanel,
  delay: DelayPanel,
  eq: EqPanel,
  filter: FilterPanel,
  flanger: FlangerPanel,
  phaser: PhaserPanel,
}

// every parameter, a group to a box, the boxes wrapping to the tab's
// width: what any built-in effect gets without a panel of its own
export const GeneratedPanel = ({ api }: { api: EffectParamsApi }) => (
  <div className='flex flex-row flex-wrap content-start gap-2 p-2'>
    {groupParams(api.params).map(group => (
      <section
        key={group.name}
        aria-label={group.name}
        className='ft-strip flex flex-col gap-2 px-2 pt-1.5 pb-2'
      >
        <h3 className='ft-label'>{group.name}</h3>
        <div className='flex flex-row flex-wrap gap-1'>
          {group.params.map(param => (
            <ParamControl
              key={param.id}
              param={param}
              // ("Output", not "Output Output")
              label={param.name === group.name ? param.name : `${group.name} ${param.name}`}
              set={api.set}
              setText={api.setText}
            />
          ))}
        </div>
      </section>
    ))}
  </div>
)

// a built-in effect's controls, in its tab
const NativePanel = ({ slotId, active = true }: { slotId: number, active?: boolean }) => {
  const api = useEffectParams(slotId)
  const { status } = api

  if(status.kind === 'loading') {
    return <div className='px-3 py-2 text-xs text-ink-faint'>Loading…</div>
  }
  if(status.kind === 'error') {
    return (
      <div className='p-3'>
        <Notice tone='danger'>Its controls couldn’t be shown: {status.message}</Notice>
      </div>
    )
  }

  const Panel = PANELS[status.effect] ?? GeneratedPanel
  return (
    <div className='flex-1 min-h-0 overflow-auto'>
      <Panel api={api} slotId={slotId} active={active} />
    </div>
  )
}

export default NativePanel
