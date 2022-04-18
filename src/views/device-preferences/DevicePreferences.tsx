import React from 'react'
import { invoke } from '@tauri-apps/api'
import PreferencePageBase from '../../components/pages/PreferencePageBase'
import Select from '../../components/common/Select'
import Option from '../../components/common/Option'

const DevicePreferences = () => {
  const [drivers, setDrivers] = React.useState<string[]>()
  const [driver, setDriver] = React.useState<string>()

  React.useEffect(() => {
    invoke('get_audio_drivers', {}).then(d => {
      setDrivers(d as string[])
      setDriver((d as string[])[0])
    })
  })

  const handleChangeDriver = (e: any) => {
    console.log("ev:",e)
    
  }

  return (
    <PreferencePageBase>
      <Select 
        value={driver}
        onChange={handleChangeDriver}
        label='Available Audio Drivers'
      >
        {drivers?.map((el: string) => (
          <Option key={el}>{el}</Option>
        ))}
      </Select>
      
    </PreferencePageBase>
  )
}

export default DevicePreferences
