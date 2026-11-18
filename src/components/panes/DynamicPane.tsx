import React from 'react'
import { Group, Layout, LayoutChangedMeta, Panel, Separator } from 'react-resizable-panels'
import { useAppDispatch } from '../../hooks/redux'
import { setSplitRatio } from '../../state/slices/windowSlice'
import { WindowNode } from '../../types/ui'
import TabbedPane from './TabbedPane'
import './styles.scss'

export interface Props {
  node: WindowNode
}

// a pane in the window layout: a split shows its two children with a
// draggable divider between them, a leaf shows its tabs
const DynamicPane = ({ node }: Props) => {
  const dispatch = useAppDispatch()
  const splitId = node.id
  const leftId = node.left?.id

  // where the divider was dragged to, kept in the layout (and so saved with
  // the project). only the user's drags: the panels taking their default
  // sizes isn't a change
  const onLayoutChanged = React.useCallback((layout: Layout, meta: LayoutChangedMeta) => {
    if(!meta.isUserInteraction || leftId === undefined) return
    const ratio = (meta.requestedLayout ?? layout)[leftId]
    if(ratio !== undefined) dispatch(setSplitRatio({ splitId, ratio }))
  }, [dispatch, splitId, leftId])

  if(!node.left || !node.right) {
    return <TabbedPane pane={node} />
  }

  // a 'vertical' split is side by side: the panels run horizontally, with a
  // vertical divider between them
  const direction = node.direction ?? 'vertical'
  const ratio = node.ratio ?? 50

  return (
    <Group
      orientation={direction === 'vertical' ? 'horizontal' : 'vertical'}
      onLayoutChanged={onLayoutChanged}
    >
      {/* keyed by id so a pane's view stays mounted when the layout around it changes */}
      <Panel key={node.left.id} id={node.left.id} defaultSize={`${ratio}%`} minSize={120}>
        <DynamicPane node={node.left} />
      </Panel>
      <Separator className={`pane-divider ${direction}`} />
      <Panel key={node.right.id} id={node.right.id} defaultSize={`${100 - ratio}%`} minSize={120}>
        <DynamicPane node={node.right} />
      </Panel>
    </Group>
  )
}

export default DynamicPane
