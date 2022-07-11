import {
  TreeDataProvider,
  TreeItem,
  TreeItemIndex,
  ExplicitDataSource,
} from 'react-complex-tree'

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
    console.log("data.items[itemId]:", this.data.items[itemId], ", data.items:",this.data.items)
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
