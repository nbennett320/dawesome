import React from 'react'
import Playlist from './playlist/Playlist'
import SamplePreview from './sample-preview/SamplePreview'
import Mixer from './mixer/Mixer'
import Transport from './transport/Transport'
import Sequencer from './sequencer/Sequencer'
import AudioSettings from './audio-settings/AudioSettings'
import Mixdown from './mixdown/Mixdown'
import Preferences from './preferences/Preferences'
import EffectsRack from './effects-rack/EffectsRack'
import PianoRoll from './piano-roll/PianoRoll'
import PluginTab from './plugin/PluginTab'
import NoteSequencer from './note-sequencer/NoteSequencer'
import { PaneTab, View } from '../types/ui'

// the component a tab shows for its view
export const matchViewComponent = ({ component: view, slotId }: PaneTab): JSX.Element => {
  switch(view) {
    case View.Playlist:
      return <Playlist />
    case View.SamplePreview:
      return <SamplePreview />
    case View.Mixer:
      return <Mixer />
    case View.Transport:
      return <Transport />
    case View.Sequencer:
      return <Sequencer />
    case View.AudioSettings:
      return <AudioSettings />
    case View.Mixdown:
      return <Mixdown />
    case View.Preferences:
      return <Preferences />
    case View.EffectsRack:
      return <EffectsRack />
    case View.PianoRoll:
      return <PianoRoll />
    case View.Plugin:
      return <PluginTab slotId={slotId as number} />
    case View.NoteSequencer:
      return <NoteSequencer />
    default:
      // eslint-disable-next-line no-console
      console.error("Tab does not have a matching component: ", view)
      return <Playlist />
  }
}

export default matchViewComponent
