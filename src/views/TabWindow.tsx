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
  selectWindowPane,
} from '../state/slices/windowSlice'
import { useAppSelector } from '../hooks/redux'
import DynamicPane from '../components/panes/DynamicPane'
import Playlist from './playlist/Playlist'
import SampleDetails from './sample-details/SampleDetails'
import { TabTypes, PaneTab, View } from '../types/ui'
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
  const [tabs, setTabs] = React.useState<PaneTab[]>([])
  const [selectedIndex, setSelectedIndex] = React.useState<number>(0)

  

  const windowPane = useAppSelector(selectWindowPane)
  console.log("tabs: ", tabs)
  console.log("window: ", windowPane)

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
              {tab.label}
            </Tab>
          ))}
        </TabList>

        {/* <TabPanel>
          {tabs.map(tab => (
            <div className='h-full w-full z-30'>
              {matchViewComponent(tab.component)}
            </div>
          ))

          }
        </TabPanel> */}
        <TabPanel>
          <div className='h-full w-full'>
            <DynamicPane 
              id='root'
              root={windowPane}
            />
          </div>
        </TabPanel>
        {/* <TabPanel>
          <div className='h-full w-full'>
            <div>sample details</div>
            <div>yoberry</div>
            <DynamicPane 
              id='root'
              root={{
                id: 'root',
                left: {
                  id: 'combined',
                  left: {
                    id: 'playlist',
                    left: {
                      id: 'playlist2',
                      child: <div>playlist</div>,
                    },
                    right: {
                      id: 'blue',
                      child: <div>blue</div>,
                    }
                  },
                  right: {
                    id: 'other',
                    child: <div>other node</div>,
                  }
                },
                right: {
                  id: 'details',
                  child: <div>details node</div>,
                }
              }}
            />
          </div>
        </TabPanel> */}
      </Tabs>
    </div>
  )
}

export default TabWindow
