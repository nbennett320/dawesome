import React from 'react'
import SplitPane from 'react-split-pane'
import { v4 as uuidv4 } from 'uuid'
import './styles.scss'

export interface Props {
  id: string
  data: Array<{
    id: string,
    child: React.ReactChild,
  }>
  onRemove?: (childId: string) => void
  onToggleParentSplit?: () => void
}

enum SplitDirection {
  Horizontal,
  Vertical,
}

export type SplitNode = {
  options?: {
    split: SplitDirection,
  }
  primaryId?: string
  secondaryId?: string
}

const DynamicPane = (props: Props) => {
  const [splitNode, setSplitNode] = React.useState<SplitNode | undefined>()
  const [nodeCount, setNodeCount] = React.useState<number>(1)

  const handleSplit = () => {
    const primaryId = uuidv4()
    const secondaryId = uuidv4()
    const newNode = {
      ...splitNode, 
      options: {
        split: SplitDirection.Vertical,
      },
      primaryId,
      secondaryId,
    }

    setSplitNode(newNode)
  }

  const renderChild = () => (
    <div>
      <div>
        child
      </div>

      <button className='bg-slate-300' onClick={handleSplit}>split</button>
    </div>
  )

  const renderHorizontalSplit = () => (
    <SplitPane
      split='horizontal'
    >
      {splitNode?.primaryId ? (
        <DynamicPane 
          id={splitNode.primaryId}
        /> 
      ) : (
        <div>Error</div>
      )}
      {splitNode?.secondaryId ? (
        <DynamicPane 
          id={splitNode.secondaryId}
        /> 
      ) : (
        <div>Error</div>
      )}
    </SplitPane>
  )

  const renderVerticalSplit = () => (
    <SplitPane
      split='vertical'
    >
      {splitNode?.primaryId ? (
        <DynamicPane 
          id={splitNode.primaryId}
          data={[
            {
              id: 'playlist',
              child: props.data[0].child
            }
          ]}
        /> 
      ) : (
        <div>Error</div>
      )}
      {splitNode?.secondaryId ? (
        <DynamicPane 
          id={splitNode.secondaryId}
        /> 
      ) : (
        <div>Error</div>
      )}
    </SplitPane>
  )

  const renderRoot = () => {
    if(splitNode?.options) {
      return splitNode.options.split === SplitDirection.Horizontal ? renderHorizontalSplit() : renderVerticalSplit()
    }

    return renderChild()
  }

  return (
    <div className='w-full'>
      {renderRoot()}
    </div>
  )
}

export default DynamicPane
