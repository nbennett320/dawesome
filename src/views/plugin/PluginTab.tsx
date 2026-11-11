import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { openEffectWindow, selectRacks, showInRack } from '../../state/slices/effectsSlice'
import { selectSynths, setSynthMuted } from '../../state/slices/sequencerSlice'
import { openTab, selectTabShowing } from '../../state/slices/windowSlice'
import { channelLabel, findSlot, hasTab } from '../../types/effects'
import { Synth } from '../../types/sequencer'
import { tabKey, View } from '../../types/ui'
import { AnalogButton, Led } from '../../components/fourtrack'
import { useDraggingTab } from '../../components/panes/dragTab'
import { PopOutIcon } from '../effects-rack/icons'
import { OnKey, SmallKey } from '../effects-rack/RackStack'
import DockedEditor from './DockedEditor'

// the key in the bar that shows where the plugin lives
const WhereKey = ({ title, onPress, children }: { title: string, onPress: () => void, children: React.ReactNode }) => (
  <AnalogButton variant='text' size='xs' className='min-w-0' title={title} onPress={onPress}>
    <span className='block truncate max-w-[16rem]'>{children}</span>
  </AnalogButton>
)

// a synth's mute: a light, and a key held down while it's muted
const MuteKey = ({ synth }: { synth: Synth }) => {
  const dispatch = useAppDispatch()

  return (
    <div className='flex flex-row items-center gap-1 shrink-0'>
      <Led color='amber' state={synth.muted ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='xs'
        latched={synth.muted}
        aria-label={`Mute ${synth.name}`}
        title={synth.muted ? 'Muted. Click to hear it again' : 'Mute it'}
        onPress={() => { dispatch(setSynthMuted(synth.id, !synth.muted)).catch(() => {}) }}
      >
        M
      </AnalogButton>
    </div>
  )
}

// one plugin's own interface, in a tab of its own: a thin bar, and the
// interface docked below. an effect's bar has its On key and its channel
// (to find it in the Effects Rack); a synth's, its mute and a key to find
// it in the Note Sequencer. both pop out to a window
const PluginTab = ({ slotId }: { slotId: number }) => {
  const dispatch = useAppDispatch()
  const racks = useAppSelector(selectRacks)
  const synths = useAppSelector(selectSynths)
  const names = useAppSelector(state => state.mixer.names)
  // tabs stay mounted while hidden: it's only docked while showing
  const showing = useAppSelector(selectTabShowing(tabKey(View.Plugin, slotId)))
  // the native view would cover the drop zones of a tab being dragged
  const dragging = useDraggingTab()

  // (its tab closes once the slot or synth is gone, or has nothing to show)
  const found = findSlot(racks, slotId)
  const synth = found ? null : synths.find(s => s.id === slotId)
  const plugin = found?.slot ?? synth
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
        {synth && (
          <>
            <MuteKey synth={synth} />
            <WhereKey title='Show it in the Note Sequencer' onPress={() => dispatch(openTab(View.NoteSequencer))}>
              Note Sequencer
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
      <DockedEditor key={String(active)} slot={plugin} active={active} />
    </div>
  )
}

export default PluginTab
