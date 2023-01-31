import React from 'react'
import { 
  Tab,
  Tabs,
  TabList,
  TabPanel
} from 'react-tabs'
import SplitPane from 'react-split-pane'
import DynamicPane from '../components/panes/DynamicPane'
import Playlist from './playlist/Playlist'
import SampleDetails from './sample-details/SampleDetails'
import './styles.scss'


interface TabData {
  tabLabels: string[]
  tabChildren: React.ReactNode[]
}

interface WindowPane {
}

// const Window = () => {
//   const [panes, setPanes] = React.useState<WindowPane[]>([])

//   return (
//     <div className='h-full'>
//       {panes.length > 1 ? (
//         <></>
//       ) : (
//         <></>
//       )}
//     </div>
//   )
// }
interface TabWindowProps {
  tabLabels: string[]
  tabChildren: React.ReactNode[]
}

const TabWindow = (props: TabWindowProps) => {
  const [selectedIndex, setSelectedIndex] = React.useState<number>(0)

  return (
    <div className='h-full'>
      <Tabs
        className='h-full'
        selectedIndex={selectedIndex}
        onSelect={(idx) => { setSelectedIndex(idx) }}
        selectedTabPanelClassName='react-tabs__tab-panel--selected h-full'
      >
        {/* <TabList className='text-xs'>
          {props.tabLabels.map(label => (
            <Tab>{label}</Tab>
          ))}
        </TabList> */}
        <TabList className='text-xs'>
          <Tab>Playlist</Tab>
          <Tab>Sample Details</Tab>
        </TabList>

        {/* {props.tabChildren.map(child => (
          <TabPanel>
            <div className='h-full w-full z-30'>
              {child}
            </div>
          </TabPanel>
        ))} */}
        <TabPanel>
          <div className='h-full w-full z-30'>
            <Playlist />
          </div>
        </TabPanel>
        <TabPanel>
          <div className='h-full w-full'>
            <div>sample details</div>
            <div>yoberry</div>
            {/* <SampleDetails /> */}
            {/* <SplitPane split='horizontal'>
              <div>yoberry3</div>
            </SplitPane> */}
            <DynamicPane 
              id='root'
              data={[
                {
                  id: 'playlist',
                  child: (<div>im the child</div>)
                }
              ]}
            />
          </div>
        </TabPanel>
      </Tabs>
    </div>
  )
}


export default TabWindow
