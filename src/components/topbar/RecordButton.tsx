import React from 'react'
import { invoke } from '@tauri-apps/api'
import { AnalogButton, RecordIcon } from '../fourtrack'

const RecordButton = () => (
  <AnalogButton
    size='sm'
    aria-label='Record'
    title='Record'
    onPress={() => { invoke('toggle_record_input', {}) }}
  >
    <RecordIcon className='ft-icon-rec' />
  </AnalogButton>
)

export default RecordButton
