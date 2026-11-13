import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { openEffectWindow, selectRacks, showInRack } from '../../state/slices/effectsSlice'
import { selectRows, setRowMuted } from '../../state/slices/sequencerSlice'
import { openTab, selectTabShowing } from '../../state/slices/windowSlice'
import { channelLabel, findSlot, hasTab } from '../../types/effects'
import { Row, SynthInstrument, synthRowBySlot } from '../../types/sequencer'
import { tabKey, View } from '../../types/ui'
import { AnalogButton, Led } from '../../components/fourtrack'
import { useDraggingTab } from '../../components/panes/dragTab'
import { OnKey, SmallKey } from '../effects-rack/RackStack'
import DockedEditor from './DockedEditor'
import { PopOutIcon } from '../../components/icons'

// the key in the bar that shows where the plugin lives
const WhereKey = ({ title, onPress, children }: { title: string, onPress: () => void, children: React.ReactNode }) => (
  <AnalogButton variant='text' size='xs' className='min-w-0' title={title} onPress={onPress}>
    <span className='block truncate max-w-[16rem]'>{children}</span>
  </AnalogButton>
)

// a synth row's mute: a light, and a key held down while it's muted
const MuteKey = ({ row }: { row: Row<SynthInstrument> }) => {
  const dispatch = useAppDispatch()

  return (
    <div className='flex flex-row items-center gap-1 shrink-0'>
      <Led color='amber' state={row.muted ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='xs'
        latched={row.muted}
        aria-label={`Mute ${row.instrument.name}`}
        title={row.muted ? 'Muted. Click to hear it again' : 'Mute it'}
        onPress={() => { dispatch(setRowMuted(row.id, !row.muted)).catch(() => {}) }}
      >
        M
      </AnalogButton>
    </div>
  )
}

// one plugin's own interface, in a tab of its own: a thin bar, and the
// interface docked below. an effect's bar has its On key and its channel
// (to find it in the Effects Rack); a synth row's, its mute and a key to
// find it in the Pattern Sequencer. both pop out to a window
const PluginTab = ({ slotId }: { slotId: number }) => {
  const dispatch = useAppDispatch()
  const racks = useAppSelector(selectRacks)
  const rows = useAppSelector(selectRows)
  const names = useAppSelector(state => state.mixer.names)
  // tabs stay mounted while hidden: it's only docked while showing
  const showing = useAppSelector(selectTabShowing(tabKey(View.Plugin, slotId)))
  // the native view would cover the drop zones of a tab being dragged
  const dragging = useDraggingTab()

  // (its tab closes once the slot or synth row is gone, or has nothing to
  // show)
  const found = findSlot(racks, slotId)
  const synthRow = found ? undefined : synthRowBySlot(rows, slotId)
  const plugin = found?.slot ?? synthRow?.instrument
  if(!plugin || !hasTab(plugin)) return null

  const active = showing && !dragging

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <div className='ft-tray shrink-0 flex flex-row items-center gap-2 px-2 py-1 border-b border-line'>
        {found && (
          <>
            <OnKey slot={found.slot} />
            <WhereKey title='Show it in the Effects Rack' onPress={() => dispatch(showInRack(slotId))}>
              {channelLabel(names, found.channel)}
            </WhereKey>
          </>
        )}
        {synthRow && (
          <>
            <MuteKey row={synthRow} />
            <WhereKey title='Show it in the Pattern Sequencer' onPress={() => dispatch(openTab(View.PatternSequencer))}>
              Pattern Sequencer
            </WhereKey>
          </>
        )}
        <div className='flex-1' />
        <SmallKey
          aria-label={`Open ${plugin.name} in a window`}
          title='Open its interface in a window'
          onPress={() => { dispatch(openEffectWindow(slotId)).catch(() => {}) }}
        >
          <PopOutIcon />
        </SmallKey>
      </div>

      {/* afresh each time it's shown */}
      <DockedEditor key={String(active)} slot={{ id: slotId, name: plugin.name }} active={active} />
    </div>
  )
}

export default PluginTab
