import React from 'react'
import { AnalogButton } from '../fourtrack'
import { SidebarPage } from '../../types/ui'
import { ClipsIcon, EffectsIcon, IconProps, InstrumentsIcon, PatternIcon, RecordingsIcon, SidebarIcon } from '../icons'

interface Props {
  open: boolean
  page: SidebarPage
  // a page's key was pressed (the sidebar decides whether that opens or closes it)
  onPress: (page: SidebarPage) => void
}

const keys: Array<{ page: SidebarPage, name: string, hint: string, Icon: React.ComponentType<IconProps> }> = [
  { page: SidebarPage.Samples, name: 'samples',
    hint: "browse audio files on disk and preview them", Icon: SidebarIcon },
  { page: SidebarPage.Patterns, name: 'patterns',
    hint: "the project's patterns, to drag into the playlist", Icon: PatternIcon },
  { page: SidebarPage.Recordings, name: 'recordings',
    hint: "takes recorded into the project's recordings folder", Icon: RecordingsIcon },
  { page: SidebarPage.Clips, name: 'clips',
    hint: "audio the app wrote out itself (deep copies and joins), in the project's clips folder", Icon: ClipsIcon },
  { page: SidebarPage.Effects, name: 'effects',
    hint: "effect plugins you can load", Icon: EffectsIcon },
  { page: SidebarPage.Instruments, name: 'instruments',
    hint: "instrument plugins you can load", Icon: InstrumentsIcon },
]

// the rail down the left: a key per sidebar page, latched while it's showing.
// pressing the latched key closes the sidebar
const SidebarList = ({ open, page, onPress }: Props) => (
  <div className='ft-panel flex flex-col items-center gap-1 px-1.5 pt-3 border-r border-line'>
    {keys.map(({ page: key, name, hint, Icon }) => {
      const showing = open && page === key

      return (
        <AnalogButton
          key={key}
          size='xs'
          latched={showing}
          aria-label={name[0].toUpperCase() + name.slice(1)}
          title={showing ? `Hide ${name}` : `Show ${name}`}
          data-hint={`${showing ? 'Hide' : 'Show'} ${name}: ${hint}`}
          onPress={() => onPress(key)}
        >
          <Icon />
        </AnalogButton>
      )
    })}
  </div>
)

export default SidebarList
