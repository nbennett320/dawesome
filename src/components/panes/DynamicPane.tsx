import React from 'react'
import SplitPane from 'react-split-pane'
import { WindowNode } from '../../types/ui'
import TabbedPane from './TabbedPane'
import './styles.scss'

export interface Props {
  node: WindowNode
}

// a pane in the window layout: a split shows its two children with a
// draggable divider between them, a leaf shows its tabs
const DynamicPane = ({ node }: Props) => {
  if(!node.left || !node.right) {
    return <TabbedPane pane={node} />
  }

  return (
    <SplitPane
      split={node.direction ?? 'vertical'}
      defaultSize='50%'
      minSize={120}
    >
      {/* keyed by id so a pane's view stays mounted when the layout around it changes */}
      <DynamicPane key={node.left.id} node={node.left} />
      <DynamicPane key={node.right.id} node={node.right} />
    </SplitPane>
  )
}

export default DynamicPane
