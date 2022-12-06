import React from 'react'
import { 
  Tab,
  Tabs,
  TabList,
  TabPanel
} from 'react-tabs'
import Playlist from './playlist/Playlist'
import SampleDetails from './sample-details/SampleDetails'
import 'react-tabs/style/react-tabs.scss'

const TabWindow = () => (
  <Tabs forceRenderTabPanel>
    <TabList>
      <Tab>Playlist</Tab>
      <Tab>Sample Details</Tab>
    </TabList>

    <TabPanel>
      <Playlist />
    </TabPanel>
    <TabPanel>
      <SampleDetails />
    </TabPanel>
  </Tabs>
)

export default TabWindow
