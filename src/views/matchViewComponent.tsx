import React from 'react'
import Playlist from './playlist/Playlist'
import SamplePreview from './sample-preview/SamplePreview'
import { View } from '../types/ui'

// the component a tab shows for its view
export const matchViewComponent = (view: View): JSX.Element => {
  switch(view) {
    case View.Playlist:
      return <Playlist />
    case View.SamplePreview:
      return <SamplePreview />
    default:
      // eslint-disable-next-line no-console
      console.error("Tab does not have a matching component: ", view)
      return <Playlist />
  }
}

export default matchViewComponent
