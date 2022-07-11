import React from 'react'
import { invoke } from '@tauri-apps/api'
import {
  Tree,
  TreeItem,
  UncontrolledTreeEnvironment,
} from 'react-complex-tree'
import { 
  getBrowserSamples,
  selectBrowserSamples,
} from '../../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../../hooks/redux'
import { 
  BrowserSampleItem,
  BrowserItemTypes,
  SampleBrowserDataProvider,
} from '../../../types/sampleBrowser'
import styles from './styles.module.scss'

interface Props {
}

const SampleBrowser = (props: React.PropsWithChildren<Props>) => {
  const samples = useAppSelector(selectBrowserSamples)
  const dispatch = useAppDispatch()
  const provider = new SampleBrowserDataProvider(
    samples, 
    (item, data) => ({ ...item, data })
  )

  React.useEffect(() => {
    dispatch(getBrowserSamples())
  }, [])

  const handlePrimaryAction = (item: TreeItem<BrowserSampleItem>, treeId: string) => {
    if(item.itemType === BrowserItemTypes.Sample) {
      invoke('preview_sample', {
        path: item.path
      })
    }
  }

  return (
    <div className={`${styles.SampleBrowser}`}>
      <div className={`${styles.SampleBrowserItemContainer} text-xs`}>
        <UncontrolledTreeEnvironment
          dataProvider={provider}
          getItemTitle={item => item.label}
          viewState={{}}
          onPrimaryAction={handlePrimaryAction}
          canDragAndDrop
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
