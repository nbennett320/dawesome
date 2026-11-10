import React from 'react'
import RecordingsBrowser from '../recordings-browser/RecordingsBrowser'

// the files the app wrote out itself (Deep copy's and Join's), in the
// project's clips folder: the recordings browser, pointed there
const ClipsBrowser = () => <RecordingsBrowser library='clips' />

export default ClipsBrowser
