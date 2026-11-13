import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { changePreferences, selectAppearance } from '../../state/slices/appearanceSlice'
import { Choice } from '../../components/common'
import type { Preferences as Prefs } from '../../theme'

const APPEARANCES = [
  { value: 'system', label: 'System' },
  { value: 'light', label: 'Light' },
  { value: 'dark', label: 'Dark' },
]

const STEP_KEYS = [
  { value: 'tr808', label: 'TR-808' },
  { value: 'tr909', label: 'TR-909' },
  { value: 'tr8s', label: 'TR-8S' },
]

// how the app looks. changes apply as they're made
const Preferences = () => {
  const dispatch = useAppDispatch()
  const { preferences, themes } = useAppSelector(selectAppearance)
  const named = (appearance: 'light' | 'dark') => themes
    .filter(t => t.appearance === appearance)
    .map(t => ({ value: t.id, label: t.name }))

  return (
    <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
      <div className='flex flex-col gap-4 max-w-xl'>
        <section className='flex flex-col gap-3'>
          <Choice
            id='preferences-appearance'
            label='Appearance'
            value={preferences.appearance}
            options={APPEARANCES}
            onChange={appearance => dispatch(changePreferences({ appearance: appearance as Prefs['appearance'] }))}
          />
          {/* both stay enabled, so either can be set ahead of time */}
          <Choice
            id='preferences-light-theme'
            label='Light theme'
            value={preferences.lightTheme}
            options={named('light')}
            onChange={lightTheme => dispatch(changePreferences({ lightTheme }))}
          />
          <Choice
            id='preferences-dark-theme'
            label='Dark theme'
            value={preferences.darkTheme}
            options={named('dark')}
            onChange={darkTheme => dispatch(changePreferences({ darkTheme }))}
          />
          <Choice
            id='preferences-step-keys'
            label='Step key style'
            value={preferences.stepKeys}
            options={STEP_KEYS}
            onChange={stepKeys => dispatch(changePreferences({ stepKeys: stepKeys as Prefs['stepKeys'] }))}
          />
        </section>
      </div>
    </div>
  )
}

export default Preferences
