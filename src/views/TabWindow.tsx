import React from 'react'
import { selectWindowPane } from '../state/slices/windowSlice'
import { useAppSelector } from '../hooks/redux'
import DynamicPane from '../components/panes/DynamicPane'
import { usePluginTabsFollowRacks } from './plugin/usePluginTabsFollowRacks'
import './styles.scss'

// the workspace: panes that can be split, each with its own tabs
const TabWindow = () => {
  const root = useAppSelector(selectWindowPane)
  usePluginTabsFollowRacks()

  return (
    // relative, since split panes position themselves inside it
    <div className='h-full w-full relative'>
      <DynamicPane node={root} />
    </div>
  )
}

export default TabWindow
