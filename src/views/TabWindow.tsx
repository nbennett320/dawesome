import React from 'react'
import { 
  Tab,
  Tabs,
  TabList,
  TabPanel
} from 'react-tabs'
import { useDrag } from 'react-dnd'
import SplitPane, { Pane } from 'react-split-pane'
import {
  removeTab,
  selectWindowPane,
} from '../state/slices/windowSlice'
import { useAppSelector, useAppDispatch } from '../hooks/redux'
import DynamicPane from '../components/panes/DynamicPane'
import Playlist from './playlist/Playlist'
import SampleDetails from './sample-details/SampleDetails'
import { TabTypes, PaneTab, View } from '../types/ui'
import Close from '../components/icons/Close'
import './styles.scss'

interface WindowPane {
}

export const matchViewComponent = (view: View): JSX.Element => {
  switch(view) {
    case View.Playlist:
      return <Playlist />
    case View.Test: 
      return <div style={{ height: '100px' }}>test component</div>
    default:
      console.error("Tab does not have matching componenr: ", view)
      return <Playlist />
  }
}

interface Props {}

const TabWindow = (props: Props) => {
  const [selectedIndex, setSelectedIndex] = React.useState<number>(0)
  const dispatch = useAppDispatch()

  const windowPane = useAppSelector(selectWindowPane)

  const closeTab = (tab: PaneTab) => {
    if(tab.index === selectedIndex) {
      setSelectedIndex(0)
    }

    dispatch(removeTab(tab, windowPane.id))
  } 

  return (
    <div className='h-full'>
      <Tabs
        className='h-full'
        selectedIndex={selectedIndex}
        onSelect={(idx) => { setSelectedIndex(idx) }}
        selectedTabPanelClassName='react-tabs__tab-panel--selected h-full'
      >
        <TabList className='text-xs'>
          {windowPane.tabs.map(tab => (
            <Tab
              key={tab.label}
              style={{ userSelect: 'none' }}
              draggable
            >
              <div className='flex row items-center justify-between'>
                <span className=''>{tab.label}</span>

                <button 
                  className='ml-1 border-0 rounded-sm hover:bg-slate-200'
                  onClick={() => { closeTab(tab) }}
                >
                  <Close
                    height={16}
                    width={16}
                    fill='rgba(0, 0, 0, 0.67)'
                  />
                </button>
              </div>
            </Tab>
          ))}
        </TabList>

        <TabPanel>
          <div className='h-full w-full'>
            <DynamicPane 
              id='root'
              root={windowPane}
            />
          </div>
        </TabPanel>
      </Tabs>
    </div>
  )
}

export default TabWindow
