import React from 'react'
import { invoke } from '@tauri-apps/api'
import PreferencePageBase from '../../components/pages/PreferencePageBase'

const DevicePreferences = () => {
  const [drivers, setDrivers] = React.useState<any>()

  React.useEffect(() => {
    invoke('get_audio_drivers', {}).then(d => {
      setDrivers(d)
    })
  })

  return (
    <PreferencePageBase>
      <h3>Available audio drivers:</h3>
      <ul>
        {drivers?.map((el: any) => (
          <li key={el}>{el}</li>
        ))}
      </ul>
    </PreferencePageBase>
  )
}

export default DevicePreferences
