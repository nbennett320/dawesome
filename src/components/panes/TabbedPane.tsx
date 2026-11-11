import React from 'react'
import { Tab, Tabs, TabList, TabPanel } from 'react-tabs'
import { useDrag, useDragLayer, useDrop } from 'react-dnd'
import {
  closeTab,
  focusPane,
  moveTab,
  selectFocusedPaneId,
  selectTab,
  selectWindowPane,
  splitPane,
} from '../../state/slices/windowSlice'
import { leaves } from '../../state/paneTree'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { matchViewComponent } from '../../views/matchViewComponent'
import { PaneTab, View, WindowNode } from '../../types/ui'
import { cssVar } from '../../theme/apply'
import Close from '../icons/Close'
import { AnalogButton, Led } from '../fourtrack'

// react-dnd type for a tab being dragged
const PANE_TAB = 'pane-tab'

interface DraggedTab {
  view: View
}

// where a tab dropped on a pane's body goes: an edge splits the pane there,
// the centre moves the tab into the pane
type DropZone = 'left' | 'right' | 'top' | 'bottom' | 'center'

// how close to an edge (as a fraction of the pane) counts as that edge
const EDGE = 0.25

const dropZoneAt = (rect: DOMRect, x: number, y: number): DropZone => {
  const rx = (x - rect.left) / rect.width
  const ry = (y - rect.top) / rect.height
  const edges: Array<[DropZone, number]> = [
    ['left', rx],
    ['right', 1 - rx],
    ['top', ry],
    ['bottom', 1 - ry],
  ]
  const [zone, dist] = edges.reduce((a, b) => (b[1] < a[1] ? b : a))

  return dist < EDGE ? zone : 'center'
}

// the part of the pane a drop zone highlights
const zoneStyle: Record<DropZone, React.CSSProperties> = {
  left: { left: 0, top: 0, bottom: 0, width: '50%' },
  right: { right: 0, top: 0, bottom: 0, width: '50%' },
  top: { left: 0, right: 0, top: 0, height: '50%' },
  bottom: { left: 0, right: 0, bottom: 0, height: '50%' },
  center: { left: 0, right: 0, top: 0, bottom: 0 },
}

interface TabItemProps {
  pane: WindowNode
  tab: PaneTab
  index: number
  selected: boolean
  // the LED: lit on the selected tab of the pane being worked in
  lit: boolean
}

// a tab's label: drag it to another pane's tabs or body, drop a tab on it
// to insert that tab before it
const TabItem = ({ pane, tab, index, selected, lit }: TabItemProps) => {
  const dispatch = useAppDispatch()

  const [, dragRef] = useDrag(() => ({
    type: PANE_TAB,
    item: { view: tab.component } as DraggedTab,
  }), [tab.component])

  const [{ isOver }, dropRef] = useDrop(() => ({
    accept: PANE_TAB,
    drop: (item: DraggedTab) => {
      dispatch(moveTab({ view: item.view, paneId: pane.id, index }))
    },
    collect: monitor => ({ isOver: monitor.isOver() }),
  }), [pane.id, index])

  return (
    <div
      ref={node => { dragRef(dropRef(node)) }}
      className='group flex flex-row items-center gap-2 h-8 pl-3 pr-1.5'
      style={{ boxShadow: isOver ? `inset 2px 0 0 ${cssVar('accent')}` : undefined }}
    >
      <Led color='blue' state={lit ? 'on' : 'off'} />
      <span className={`ft-label ${selected ? 'text-ink' : 'group-hover:text-ink-mid'}`}>{tab.label}</span>

      <button
        className={`rounded-sm p-0.5 text-ink-faint hover:bg-sunken hover:text-ink ${
          selected ? '' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'
        }`}
        title={`Close ${tab.label}`}
        aria-label={`Close ${tab.label}`}
        onClick={ev => {
          // don't select the tab being closed
          ev.stopPropagation()
          dispatch(closeTab(tab.component))
        }}
      >
        <Close height={12} width={12} fill='currentColor' />
      </button>
    </div>
  )
}

// the space after a pane's tabs: dropping a tab here adds it at the end
const TabListEnd = ({ pane }: { pane: WindowNode }) => {
  const dispatch = useAppDispatch()

  const [, dropRef] = useDrop(() => ({
    accept: PANE_TAB,
    drop: (item: DraggedTab) => {
      dispatch(moveTab({ view: item.view, paneId: pane.id }))
    },
  }), [pane.id])

  return <div ref={dropRef} className='flex-1 self-stretch' />
}

// shown over a pane's body while a tab is dragged, to split the pane or
// move the tab into it
const DropOverlay = ({ pane }: { pane: WindowNode }) => {
  const dispatch = useAppDispatch()
  const ref = React.useRef<HTMLDivElement | null>(null)
  const [zone, setZone] = React.useState<DropZone | null>(null)

  // whether dropping in a zone would change anything: a pane's only tab
  // can't be split away from it, or moved into the pane it's already in
  const canDropIn = (z: DropZone, view: View) => {
    const own = pane.tabs.some(t => t.component === view)
    if(z === 'center') return !own
    return !own || pane.tabs.length > 1
  }

  const zoneFor = (item: DraggedTab, offset: { x: number, y: number } | null) => {
    if(!ref.current || !offset) return null
    const z = dropZoneAt(ref.current.getBoundingClientRect(), offset.x, offset.y)
    return canDropIn(z, item.view) ? z : null
  }

  const [{ isOver }, dropRef] = useDrop(() => ({
    accept: PANE_TAB,
    hover: (item: DraggedTab, monitor) => {
      setZone(zoneFor(item, monitor.getClientOffset()))
    },
    drop: (item: DraggedTab, monitor) => {
      const z = zoneFor(item, monitor.getClientOffset())

      if(z === 'center') {
        dispatch(moveTab({ view: item.view, paneId: pane.id }))
      } else if(z) {
        const direction = z === 'left' || z === 'right' ? 'vertical' : 'horizontal'
        const before = z === 'left' || z === 'top'
        dispatch(splitPane(pane.id, item.view, direction, before))
      }
    },
    collect: monitor => ({ isOver: monitor.isOver() }),
  }), [pane])

  return (
    <div
      ref={node => {
        ref.current = node
        dropRef(node)
      }}
      className='absolute inset-0'
      style={{ zIndex: 50 }}
    >
      {isOver && zone && (
        <div
          className='absolute pointer-events-none'
          style={{
            ...zoneStyle[zone],
            background: cssVar('accentSoft', 0.2),
            border: `1px solid ${cssVar('accent', 0.6)}`,
            borderRadius: 4,
          }}
        />
      )}
    </div>
  )
}

// split icons for the tab bar
const SplitIcon = ({ direction }: { direction: 'vertical' | 'horizontal' }) => (
  <svg viewBox='0 0 16 16' fill='none' stroke='currentColor' strokeWidth='1.25'>
    <rect x='1.5' y='2.5' width='13' height='11' rx='1' />
    {direction === 'vertical'
      ? <line x1='8' y1='2.5' x2='8' y2='13.5' />
      : <line x1='1.5' y1='8' x2='14.5' y2='8' />}
  </svg>
)

interface Props {
  pane: WindowNode
}

// a leaf pane: its tabs, and the selected tab's view
//
// every tab's view stays mounted while another tab is selected, so the
// playlist doesn't rebuild its canvas when switching tabs
const TabbedPane = ({ pane }: Props) => {
  const dispatch = useAppDispatch()
  const root = useAppSelector(selectWindowPane)
  const focusedPaneId = useAppSelector(selectFocusedPaneId)
  const draggingTab = useDragLayer(monitor =>
    monitor.isDragging() && monitor.getItemType() === PANE_TAB)

  const selectedIndex = Math.max(
    0, pane.tabs.findIndex(t => t.component === pane.activeView))
  const split = leaves(root).length > 1
  const focused = split && pane.id === focusedPaneId

  // move the selected tab into a new pane beside this one
  const splitSelected = (direction: 'vertical' | 'horizontal') => {
    const tab = pane.tabs[selectedIndex]
    if(tab) dispatch(splitPane(pane.id, tab.component, direction))
  }
  const canSplit = pane.tabs.length > 1

  return (
    <div
      className='h-full w-full flex flex-col'
      onMouseDownCapture={() => {
        if(pane.id !== focusedPaneId) dispatch(focusPane(pane.id))
      }}
    >
      <Tabs
        className='h-full flex flex-col'
        selectedIndex={selectedIndex}
        onSelect={idx => {
          const tab = pane.tabs[idx]
          if(tab) dispatch(selectTab({ paneId: pane.id, view: tab.component }))
        }}
        forceRenderTabPanel
        selectedTabPanelClassName='react-tabs__tab-panel--selected h-full'
      >
        <div
          className='ft-panel flex flex-row items-stretch border-b border-line'
        >
          <TabList className='flex flex-row' style={{ margin: 0, border: 'none' }}>
            {pane.tabs.map((tab, index) => (
              <Tab
                key={tab.component}
                style={{ userSelect: 'none', padding: 0 }}
              >
                <TabItem
                  pane={pane}
                  tab={tab}
                  index={index}
                  selected={index === selectedIndex}
                  lit={index === selectedIndex && (focused || !split)}
                />
              </Tab>
            ))}
          </TabList>

          <TabListEnd pane={pane} />

          <div className='flex flex-row items-center gap-1.5 px-2 pt-0.5'>
            <AnalogButton
              size='xs'
              className='ft-btn--tab'
              title='Split right'
              aria-label='Split right'
              disabled={!canSplit}
              onPress={() => splitSelected('vertical')}
            >
              <SplitIcon direction='vertical' />
            </AnalogButton>
            <AnalogButton
              size='xs'
              className='ft-btn--tab'
              title='Split down'
              aria-label='Split down'
              disabled={!canSplit}
              onPress={() => splitSelected('horizontal')}
            >
              <SplitIcon direction='horizontal' />
            </AnalogButton>
          </div>
        </div>

        <div className='flex-1 min-h-0 relative overflow-hidden'>
          {pane.tabs.map(tab => (
            <TabPanel key={tab.component}>
              {matchViewComponent(tab.component)}
            </TabPanel>
          ))}

          {draggingTab && <DropOverlay pane={pane} />}
        </div>
      </Tabs>
    </div>
  )
}

export default TabbedPane
