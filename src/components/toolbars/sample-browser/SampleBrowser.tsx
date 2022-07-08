import React from 'react'
import {
  StaticTreeDataProvider,
  TreeDataProvider,
  Tree,
  TreeItem,
  UncontrolledTreeEnvironment,
  TreeItemIndex,
  ExplicitDataSource,
} from 'react-complex-tree'
import { 
  getBrowserSamples,
  selectBrowserSamples,
} from '../../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../../hooks/redux'
import SampleItem from './SampleItem'
import { BrowserSampleItem } from '../../../types/sampleBrowser'
import styles from './styles.module.scss'

interface Props {
}

class SampleBrowserDataProvider<T = BrowserSampleItem> implements TreeDataProvider {
  private data: ExplicitDataSource

  private setItemName?: (item: TreeItem<T>, newName: string) => TreeItem<T>

  constructor(
    items: Record<TreeItemIndex, TreeItem<T>>, 
    setItemName: (item: TreeItem<T>, newName: string) => TreeItem<T>,
  ) {
    console.log("constructor:",items)
    this.data = { items }
    this.setItemName = setItemName
  }

  public async getTreeItem (itemId: TreeItemIndex): Promise<TreeItem> {
    console.log("getTreeItem: ", itemId)
    // eslint-disable-next-line no-debugger
    console.log(this.data.items[itemId])
    return this.data.items[itemId]
  }

  // eslint-disable-next-line class-methods-use-this
  // public onChangeItemChildren (
  //   itemId: TreeItemIndex, 
  //   newChildren: TreeItemIndex[]
  // ): Promise<void> {
  //   console.log(itemId)
  //   console.log(newChildren)

  //   // this.data.items = { data.items, ...newChildren }
  //   const res = new Promise<void>((val) => {})
  //   return res
  // }
}

const SampleBrowser = (props: React.PropsWithChildren<Props>) => {
  const samples = useAppSelector(selectBrowserSamples)
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    dispatch(getBrowserSamples())
  }, [])

  return (
    <div className={`${styles.SampleBrowser}`}>
      <div className={styles.SampleBrowserItemContainer}>
        <UncontrolledTreeEnvironment
          dataProvider={new SampleBrowserDataProvider(samples, (item, data) => ({ ...item, data }))}
          getItemTitle={item => item.data}
          viewState={{}}
        >
          <Tree 
            treeId='sample-browser'
            rootItem='root'
            treeLabel='Samples'
          />
        </UncontrolledTreeEnvironment>
      </div>
    </div>
  )
}

export default SampleBrowser
