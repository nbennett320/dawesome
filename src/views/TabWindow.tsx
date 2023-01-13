import React from 'react'
import { 
  Tab,
  Tabs,
  TabList,
  TabPanel
} from 'react-tabs'
import Playlist from './playlist/Playlist'
import SampleDetails from './sample-details/SampleDetails'
import './styles.scss'

const TabWindow = () => {
  const [selectedIndex, setSelectedIndex] = React.useState<number>(0)

  return (
    <div className='h-full'>
      <Tabs
        className='h-full'
        selectedIndex={selectedIndex}
        onSelect={(idx) => { setSelectedIndex(idx) }}
        selectedTabPanelClassName='react-tabs__tab-panel--selected h-full'
      >
        <TabList className='text-xs'>
          <Tab>Playlist</Tab>
          <Tab>Sample Details</Tab>
        </TabList>

        <TabPanel>
          <div className='h-full w-full z-30'>
            <Playlist />
          </div>
        </TabPanel>
        <TabPanel>
          <div>sample details</div>
          {/* <SampleDetails /> */}
        </TabPanel>
      </Tabs>
    </div>
  )
}

export default TabWindow
