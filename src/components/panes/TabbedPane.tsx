import React from 'react'
import { Tab, Tabs, TabList, TabPanel } from 'react-tabs'
import { useDrag, useDrop } from 'react-dnd'
import {
  closeTab,
  focusPane,
  moveTab,
  selectFocusedPaneId,
  selectTab,
  selectTabLabel,
  selectWindowPane,
  splitPane,
} from '../../state/slices/windowSlice'
import { leaves } from '../../state/paneTree'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { matchViewComponent } from '../../views/matchViewComponent'
import { keyOf, PaneTab, TabKey, WindowNode } from '../../types/ui'
import { cssVar } from '../../theme/apply'
import { AnalogButton, Led } from '../fourtrack'
import { DraggedTab, PANE_TAB, useDraggingTab } from './dragTab'
import { CloseIcon, SplitIcon } from '../icons'

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
  const key = keyOf(tab)
  // worked out each time: a plugin's tab is named after the others like it
  const label = useAppSelector(selectTabLabel(tab))

  const [, dragRef] = useDrag(() => ({
    type: PANE_TAB,
    item: { key } as DraggedTab,
  }), [key])

  const [{ isOver }, dropRef] = useDrop(() => ({
    accept: PANE_TAB,
    drop: (item: DraggedTab) => {
      dispatch(moveTab({ key: item.key, paneId: pane.id, index }))
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
      <span className={`ft-label ${selected ? 'text-ink' : 'group-hover:text-ink-mid'}`}>{label}</span>

      <button
        className={`rounded-sm p-0.5 text-ink-faint hover:bg-sunken hover:text-ink ${
          selected ? '' : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100'
        }`}
        title={`Close ${label}`}
        aria-label={`Close ${label}`}
        onClick={ev => {
          // don't select the tab being closed
          ev.stopPropagation()
          dispatch(closeTab(key))
        }}
      >
        <CloseIcon height={12} width={12} />
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
      dispatch(moveTab({ key: item.key, paneId: pane.id }))
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
  const canDropIn = (z: DropZone, key: TabKey) => {
    const own = pane.tabs.some(t => keyOf(t) === key)
    if(z === 'center') return !own
    return !own || pane.tabs.length > 1
  }

  const zoneFor = (item: DraggedTab, offset: { x: number, y: number } | null) => {
    if(!ref.current || !offset) return null
    const z = dropZoneAt(ref.current.getBoundingClientRect(), offset.x, offset.y)
    return canDropIn(z, item.key) ? z : null
  }

  const [{ isOver }, dropRef] = useDrop(() => ({
    accept: PANE_TAB,
    hover: (item: DraggedTab, monitor) => {
      setZone(zoneFor(item, monitor.getClientOffset()))
    },
    drop: (item: DraggedTab, monitor) => {
      const z = zoneFor(item, monitor.getClientOffset())

      if(z === 'center') {
        dispatch(moveTab({ key: item.key, paneId: pane.id }))
      } else if(z) {
        const direction = z === 'left' || z === 'right' ? 'vertical' : 'horizontal'
        const before = z === 'left' || z === 'top'
        dispatch(splitPane(pane.id, item.key, direction, before))
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
  const draggingTab = useDraggingTab()

  const selectedIndex = Math.max(
    0, pane.tabs.findIndex(t => keyOf(t) === pane.activeTab))
  const split = leaves(root).length > 1
  const focused = split && pane.id === focusedPaneId

  // move the selected tab into a new pane beside this one
  const splitSelected = (direction: 'vertical' | 'horizontal') => {
    const tab = pane.tabs[selectedIndex]
    if(tab) dispatch(splitPane(pane.id, keyOf(tab), direction))
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
          if(tab) dispatch(selectTab({ paneId: pane.id, key: keyOf(tab) }))
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
                key={keyOf(tab)}
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
            <TabPanel key={keyOf(tab)}>
              {matchViewComponent(tab)}
            </TabPanel>
          ))}

          {draggingTab && <DropOverlay pane={pane} />}
        </div>
      </Tabs>
    </div>
  )
}

export default TabbedPane
