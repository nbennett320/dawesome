import React from 'react'
import { invoke } from '@tauri-apps/api'
import PreferencePageBase from '../../components/pages/PreferencePageBase'
import Select from '../../components/common/Select'
import Option from '../../components/common/Option'

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
      <Select>
        {drivers?.map((el: any) => (
          <Option key={el}>{el}</Option>
        ))}
      </Select>
      
    </PreferencePageBase>
  )
}

export default DevicePreferences
