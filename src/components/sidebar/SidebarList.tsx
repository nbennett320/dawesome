import React from 'react'
import SidebarIcon from '../icons/SidebarIcon'
import PatternIcon from '../icons/PatternIcon'
import RecordingsIcon from '../icons/RecordingsIcon'
import ClipsIcon from '../icons/ClipsIcon'
import { AnalogButton } from '../fourtrack'
import { SidebarPage } from '../../types/ui'

interface Props {
  open: boolean
  page: SidebarPage
  // a page's key was pressed (the sidebar decides whether that opens or closes it)
  onPress: (page: SidebarPage) => void
}

const keys: Array<{ page: SidebarPage, name: string, Icon: React.ComponentType<React.SVGAttributes<object>> }> = [
  { page: SidebarPage.Samples, name: 'samples', Icon: SidebarIcon },
  { page: SidebarPage.Patterns, name: 'patterns', Icon: PatternIcon },
  { page: SidebarPage.Recordings, name: 'recordings', Icon: RecordingsIcon },
  { page: SidebarPage.Clips, name: 'clips', Icon: ClipsIcon },
]

// the rail down the left: a key per sidebar page, latched while it's showing.
// pressing the latched key closes the sidebar
const SidebarList = ({ open, page, onPress }: Props) => (
  <div className='ft-panel flex flex-col items-center gap-1 px-1.5 pt-3 border-r border-line'>
    {keys.map(({ page: key, name, Icon }) => {
      const showing = open && page === key

      return (
        <AnalogButton
          key={key}
          size='xs'
          latched={showing}
          aria-label={name[0].toUpperCase() + name.slice(1)}
          title={showing ? `Hide ${name}` : `Show ${name}`}
          onPress={() => onPress(key)}
        >
          <Icon fill='currentColor' />
        </AnalogButton>
      )
    })}
  </div>
)

export default SidebarList
