import React from 'react'
import { Group, Panel, Separator } from 'react-resizable-panels'
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

  // a 'vertical' split is side by side: the panels run horizontally, with a
  // vertical divider between them
  const direction = node.direction ?? 'vertical'

  return (
    <Group orientation={direction === 'vertical' ? 'horizontal' : 'vertical'}>
      {/* keyed by id so a pane's view stays mounted when the layout around it changes */}
      <Panel key={node.left.id} id={node.left.id} defaultSize='50%' minSize={120}>
        <DynamicPane node={node.left} />
      </Panel>
      <Separator className={`pane-divider ${direction}`} />
      <Panel key={node.right.id} id={node.right.id} defaultSize='50%' minSize={120}>
        <DynamicPane node={node.right} />
      </Panel>
    </Group>
  )
}

export default DynamicPane
