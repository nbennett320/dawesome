import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { changePreferences, selectAppearance } from '../../state/slices/appearanceSlice'
import { Choice } from '../../components/common'
import { AnalogButton, Led } from '../../components/fourtrack'
import type { Preferences as Prefs } from '../../theme'

const PLUGIN_VIEWS = [
  { value: 'window', label: 'In a window of their own' },
  { value: 'tab', label: 'In a tab' },
]

// how the app is worked: where plugins open, and the command bar. changes
// apply as they're made
const Controls = () => {
  const dispatch = useAppDispatch()
  const { preferences } = useAppSelector(selectAppearance)
  const { commandBar } = preferences

  return (
    <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
      <div className='flex flex-col gap-4 max-w-xl'>
        <section className='flex flex-col gap-3'>
          <Choice
            id='preferences-plugin-view'
            label='Plugins open'
            value={preferences.pluginView}
            options={PLUGIN_VIEWS}
            onChange={pluginView => dispatch(changePreferences({ pluginView: pluginView as Prefs['pluginView'] }))}
          />
          <div className='flex flex-col gap-1'>
            <span className='ft-label'>Command bar</span>
            <div className='flex flex-row items-center gap-2'>
              <AnalogButton
                size='xs'
                latched={commandBar}
                aria-label='Command bar'
                data-hint='A vim-style command bar: type : to open it along the bottom. See the docs for the available commands'
                onPress={() => dispatch(changePreferences({ commandBar: !commandBar }))}
              />
              <Led color='blue' state={commandBar ? 'on' : 'off'} />
              <span>{commandBar ? 'On' : 'Off'}</span>
            </div>
          </div>
        </section>
      </div>
    </div>
  )
}

export default Controls
