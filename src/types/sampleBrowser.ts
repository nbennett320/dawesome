import {
  TreeDataProvider,
  TreeItem,
  TreeItemIndex,
  ExplicitDataSource,
  Disposable,
} from 'react-complex-tree'
import { EventEmitter } from '../util/events'

export enum BrowserItemTypes {
  Directory = 'directory',
  Sample = 'sample',
}

export type BrowserSampleItem = {
  index: string,
  canMove: boolean,
  hasChildren: boolean,
  children: string[],
  data: string,
  label: string,
  path: string,
  canRename: boolean,
  itemType: BrowserItemTypes,
}

export class SampleBrowserDataProvider<T = BrowserSampleItem> implements TreeDataProvider {
  private data: ExplicitDataSource

  private setItemName?: (item: TreeItem<T>, newName: string) => TreeItem<T>

  private onDidChangeTreeDataEmitter = new EventEmitter<TreeItemIndex[]>()

  constructor(
    items: Record<TreeItemIndex, TreeItem<T>>, 
    setItemName: (item: TreeItem<T>, newName: string) => TreeItem<T>,
  ) {
    this.data = { items }
    this.setItemName = setItemName
  }

  public async getTreeItem (itemId: TreeItemIndex): Promise<TreeItem> {
    return this.data.items[itemId]
  }

  // eslint-disable-next-line class-methods-use-this
  public onChangeItemChildren (
    itemId: TreeItemIndex, 
    newChildren: TreeItemIndex[]
  ): Promise<void> {
    // this.data.items = { data.items, ...newChildren }
    const res = new Promise<void>((val) => {})
    return res
  }

  // eslint-disable-next-line class-methods-use-this
  public onDidChangeTreeData(listener: (changedItemIds: TreeItemIndex[]) => void): Disposable {
    const handlerId = this.onDidChangeTreeDataEmitter.on(payload => listener(payload));
    return { dispose: () => this.onDidChangeTreeDataEmitter.off(handlerId) };
  }

}
