import { TreeDataProvider, TreeItem, TreeItemIndex } from 'react-complex-tree'

export type BrowserSampleItem = {
  index: string,
  canMove: boolean,
  hasChildren: boolean,
  children: string[],
  data: string,
  canRename: boolean,
}

// class SampleBrowserDataProvider implements TreeDataProvider<BrowserSampleItem> {
//   getTreeItem: (itemId: TreeItemIndex) => {

//   }
// }

// export const SampleBrowserDataProvider = {
//   // onDidChangeTreeData: (listener: (changedItemIds: TreeItemIndex[]) => void) => Disposable,
//   // getTreeItem: (itemId: TreeItemIndex) => Promise<TreeItem<BrowserSampleItem>>,
//   // getTreeItem: (itemId: TreeItemIndex) => {

//   // },
//   // getTreeItems?: (itemIds: TreeItemIndex[]) => Promise<TreeItem[]>,
//   // onRenameItem?: (item: TreeItem<T>, name: string) => Promise<void>,
//   // onChangeItemChildren?: (itemId: TreeItemIndex, newChildren: TreeItemIndex[]) => Promise<void>,
// } as TreeDataProvider<BrowserSampleItem>
