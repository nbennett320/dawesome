import React from 'react'
import SplitPane from 'react-split-pane'
import { WindowNode } from '../../types/ui'
import './styles.scss'


export interface Props {
  id: string
  root: WindowNode
  onRemove?: (childId: string) => void
  onToggleParentSplit?: () => void
}

const LeftChildError = () => (
  <div>Error rendering left child</div>
)

const DynamicPane = (props: Props) => {
  console.log("dynamic pane props '" + props.root.id + "': ", props)
  const ref = React.useRef<HTMLDivElement>(null)
  const [mouseLeft, setMouseLeft] = React.useState()

  const handleMouseEnter = (e: MouseEvent) => {
    console.log("ref:", ref)

    if(ref.current) {
      const { width, height, left } = ref.current.getBoundingClientRect()

      if(e.clientX > left) {
        console.log("left!!!", e.clientX, left)
      }

    }
  }

  const renderRoot = () => {
    const { left, right } = props.root

    if(left?.child && right?.child) {
      // render left and right children
      return (
        <SplitPane split='vertical'>
          {left.child}
          {right.child}
        </SplitPane>
      )
    }

    if(left?.child && right && !right?.child) {
      // render left child and right root
      return (
        <SplitPane split='vertical'>
          {left.child}

          <DynamicPane 
            id={right.id}
            root={right}
          />
        </SplitPane>
      )
    }

    if(left && !left?.child && right?.child) {
      // render left tree and right children
      return (
        <SplitPane split='vertical'>
          <DynamicPane 
            id={left.id}
            root={left}
          />

          {right.child}
        </SplitPane>
      )
    }

    if(left && !left?.child && right && !right?.child) {
      // render left and right tree
      return (
        <SplitPane split='vertical'>
          <DynamicPane 
            id={left.id}
            root={left}
          />

          <DynamicPane 
            id={right.id}
            root={right}
          />
        </SplitPane>
      )
    }

    return (
      <div>Error rendering pane tree</div>
    )
  }

  return (
    <div 
      ref={ref}
      onMouseEnter={(e) => { handleMouseEnter(e) }}
      className='w-full'
    >
      {renderRoot()}
    </div>
  )
}

export default DynamicPane
