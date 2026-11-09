import React from 'react'
import { invoke } from '@tauri-apps/api'
import { TreeInformation, TreeItemRenderContext } from 'react-complex-tree'
import { getDirectorySamples } from '../../../state/slices/envSlice'
import { useAppDispatch } from '../../../hooks/redux'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { BrowserItemTypes, BrowserSampleItem } from '../../../types/sampleBrowser'
import styles from './styles.module.scss'

interface Props {
  item: BrowserSampleItem,
  label: React.ReactNode,
  arrow: React.ReactNode,
  context: TreeItemRenderContext,
  depth: number,
  info: TreeInformation,
}

const SampleItem = (props: Props) => {
  const dispatch = useAppDispatch()
  const previewSample = () => {
    console.log("playing")
    invoke('t_preview_sample', {
      path: props.item.path
    })
  }

  const handleClick = (ev: React.MouseEvent) => {
    if(props.item.itemType === BrowserItemTypes.Sample) {
      invoke('t_preview_sample', {
        path: props.item.path
      })
      // dragging the sample into the playlist adds it there
      beginSampleDrag(props.item.path, ev)
    } else if(props.item.itemType === BrowserItemTypes.Directory) {
      console.log("enumerating directory")
      dispatch(getDirectorySamples(props.item.path))
      // setExpandedItems([...expandedItems, treeId])
    }
  }

  return (
    <button
      className={`${styles.SampleItem} text-xs text-ellipsis whitespace-nowrap overflow-hidden w-full`}
      onMouseDown={handleClick}
    >
      {props.item.itemType === BrowserItemTypes.Directory && props.arrow} {props.label}
    </button>
  )
}

export default SampleItem
