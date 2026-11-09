import React from 'react'
import SidebarIcon from '../icons/SidebarIcon'
import { AnalogButton, Led } from '../fourtrack'
import { SidebarPage } from './Sidebar'

interface Props {
  open: boolean
  page: SidebarPage
  onChangePage: (next: SidebarPage) => void
  onClose: () => void
}

// the rail down the left: a key per sidebar page, latched while it's open
const SidebarList = (props: React.PropsWithChildren<Props>) => {
  const handleClick = (page: SidebarPage) => {
    if(props.open && props.page === page) {
      props.onClose()
    } else if(props.open) {
      props.onChangePage(page)
    } else {
      props.onClose()
    }
  }

  const samplesOpen = props.open && props.page === SidebarPage.FileBrowser

  return (
    <div className='ft-panel flex flex-col items-center gap-1 px-1.5 pt-3 border-r border-slate-200'>
      <AnalogButton
        size='xs'
        latched={samplesOpen}
        aria-label='Samples'
        title={samplesOpen ? 'Hide samples' : 'Show samples'}
        onPress={() => handleClick(SidebarPage.FileBrowser)}
      >
        <SidebarIcon fill='currentColor' />
      </AnalogButton>
    </div>
  )
}

export default SidebarList
