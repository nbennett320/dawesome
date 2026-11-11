import { useDragLayer } from 'react-dnd'
import { TabKey } from '../../types/ui'

// react-dnd type for a tab being dragged
export const PANE_TAB = 'pane-tab'

export interface DraggedTab {
  key: TabKey
}

// whether a tab is being dragged, e.g. to hide what would cover the panes'
// drop zones
export const useDraggingTab = (): boolean =>
  useDragLayer(monitor => monitor.isDragging() && monitor.getItemType() === PANE_TAB)
