import React from 'react'
import Playlist from './playlist/Playlist'
import SamplePreview from './sample-preview/SamplePreview'
import Mixer from './mixer/Mixer'
import Transport from './transport/Transport'
import { View } from '../types/ui'

// the component a tab shows for its view
export const matchViewComponent = (view: View): JSX.Element => {
  switch(view) {
    case View.Playlist:
      return <Playlist />
    case View.SamplePreview:
      return <SamplePreview />
    case View.Mixer:
      return <Mixer />
    case View.Transport:
      return <Transport />
    default:
      // eslint-disable-next-line no-console
      console.error("Tab does not have a matching component: ", view)
      return <Playlist />
  }
}

export default matchViewComponent
