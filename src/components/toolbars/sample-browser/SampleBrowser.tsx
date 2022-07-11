import React from 'react'
import { invoke } from '@tauri-apps/api'
import {
  Tree,
  TreeItem,
  ControlledTreeEnvironment,
  TreeItemIndex,
} from 'react-complex-tree'
import { 
  getBrowserRootSamples,
  getDirectorySamples,
  selectBrowserSamples,
} from '../../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../../hooks/redux'
import { 
  BrowserSampleItem,
  BrowserItemTypes,
  SampleBrowserDataProvider,
} from '../../../types/sampleBrowser'
import styles from './styles.module.scss'
import ChevronUpDown from '../../icons/ChevronUpDown'

interface Props {
}

const SampleBrowser = (props: React.PropsWithChildren<Props>) => {
  const samples = useAppSelector(selectBrowserSamples)
  const dispatch = useAppDispatch()
  // const provider = new SampleBrowserDataProvider(
  //   samples, 
  //   (item, data) => ({ ...item, data })
  // )
  const [focusedItem, setFocusedItem] = React.useState<TreeItemIndex>();
  const [expandedItems, setExpandedItems] = React.useState<TreeItemIndex[]>([]);

  React.useEffect(() => {
    dispatch(getBrowserRootSamples())
  }, [])

  React.useEffect(() => {
    console.log("expanded an item", expandedItems)
  }, [expandedItems])

  const handlePrimaryAction = (item: TreeItem<BrowserSampleItem>, treeId: string) => {
    if(item.itemType === BrowserItemTypes.Sample) {
      invoke('preview_sample', {
        path: item.path
      })
    } else if(item.itemType === BrowserItemTypes.Directory) {
      console.log("enumerating directory")
      dispatch(getDirectorySamples(item.path))
      // setExpandedItems([...expandedItems, treeId])
    }
  }

  return (
    <div className={`${styles.SampleBrowser}`}>
      <div className={`${styles.SampleBrowserItemContainer} text-xs`}>
        <ControlledTreeEnvironment
          items={samples}
          // dataProvider={provider}
          getItemTitle={item => item.label}
          viewState={{
            tree: {
              focusedItem,
              expandedItems,
            }
          }}
          onPrimaryAction={handlePrimaryAction}
          onFocusItem={item => setFocusedItem(item.index)}
          onExpandItem={item => setExpandedItems([...expandedItems, item.index])}
          onCollapseItem={item => setExpandedItems([...expandedItems.filter(e => e !== item.index)])}
          onMissingItems={items => console.log("missing: ",items)}
          renderItemArrow={({ item, context }) =>
            // eslint-disable-next-line no-nested-ternary
            item.hasChildren ? context.isExpanded ? <span>{'v '}</span> : <span>{'> '}</span> : null
          }
          canDragAndDrop
        >
          <Tree 
            treeId='sample-browser'
            rootItem='root'
            treeLabel='Samples'
          />
        </ControlledTreeEnvironment>
      </div>
    </div>
  )
}

export default SampleBrowser
