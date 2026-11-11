import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { openEffectWindow, selectRacks, showInRack } from '../../state/slices/effectsSlice'
import { selectTabShowing } from '../../state/slices/windowSlice'
import { channelLabel, findSlot, hasTab } from '../../types/effects'
import { tabKey, View } from '../../types/ui'
import { AnalogButton } from '../../components/fourtrack'
import { useDraggingTab } from '../../components/panes/dragTab'
import { PopOutIcon } from '../effects-rack/icons'
import { OnKey, SmallKey } from '../effects-rack/RackStack'
import DockedEditor from './DockedEditor'

// one effect's own interface, in a tab of its own: a thin bar with its On
// key, its channel (to find it in the Effects Rack) and popping it out, and
// the interface docked below
const PluginTab = ({ slotId }: { slotId: number }) => {
  const dispatch = useAppDispatch()
  const racks = useAppSelector(selectRacks)
  const names = useAppSelector(state => state.mixer.names)
  // tabs stay mounted while hidden: it's only docked while showing
  const showing = useAppSelector(selectTabShowing(tabKey(View.Plugin, slotId)))
  // the native view would cover the drop zones of a tab being dragged
  const dragging = useDraggingTab()

  // (its tab closes once the slot's gone, or has nothing to show)
  const found = findSlot(racks, slotId)
  if(!found || !hasTab(found.slot)) return null

  const { slot, channel } = found
  const active = showing && !dragging

  return (
    <div className='ft-panel h-full w-full flex flex-col text-sm select-none'>
      <div className='ft-tray shrink-0 flex flex-row items-center gap-2 px-2 py-1 border-b border-line'>
        <OnKey slot={slot} />
        <AnalogButton
          variant='text'
          size='xs'
          className='min-w-0'
          title='Show it in the Effects Rack'
          onPress={() => dispatch(showInRack(slot.id))}
        >
          <span className='block truncate max-w-[16rem]'>{channelLabel(names, channel)}</span>
        </AnalogButton>
        <div className='flex-1' />
        <SmallKey
          aria-label={`Open ${slot.name} in a window`}
          title='Open its interface in a window'
          onPress={() => { dispatch(openEffectWindow(slot.id)).catch(() => {}) }}
        >
          <PopOutIcon />
        </SmallKey>
      </div>

      {/* afresh each time it's shown */}
      <DockedEditor key={String(active)} slot={slot} active={active} />
    </div>
  )
}

export default PluginTab
