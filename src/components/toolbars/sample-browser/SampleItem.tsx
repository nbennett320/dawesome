import React from 'react'
import { invoke } from '@tauri-apps/api'
import { useDrag } from 'react-dnd'
import { TreeInformation, TreeItemRenderContext } from 'react-complex-tree'
import { getDirectorySamples } from '../../../state/slices/envSlice'
import { useAppDispatch } from '../../../hooks/redux'
import { PlaylistTypes } from '../../../types/playlist'
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
  const [{ isDragging }, drag] = useDrag(() => ({
    type: PlaylistTypes.SidebarSampleItem,
    item: { name: props.item.path },
    end: (item, monitor) => {
      const dropResult = monitor.getDropResult<Props>()
      if (item && dropResult) {
        console.log("dropped, from sample item", dropResult)
      }
    },
    collect: (monitor) => ({
      isDragging: monitor.isDragging(),
      handlerId: monitor.getHandlerId(),
    }),
  }))

  if(isDragging) console.log("dragging sample item")

  const previewSample = () => {
    console.log("playing")
    invoke('preview_sample', {
      path: props.item.path
    })
  }

  const handleClick = () => {
    if(props.item.itemType === BrowserItemTypes.Sample) {
      invoke('preview_sample', {
        path: props.item.path
      })
    } else if(props.item.itemType === BrowserItemTypes.Directory) {
      console.log("enumerating directory")
      dispatch(getDirectorySamples(props.item.path))
      // setExpandedItems([...expandedItems, treeId])
    }
  }

  return (
    <button
      ref={drag}
      className={`${styles.SampleItem} text-xs text-ellipsis whitespace-nowrap overflow-hidden w-full`}
      onMouseDown={handleClick}
    >
      {props.item.itemType === BrowserItemTypes.Directory && props.arrow} {props.label}
    </button>
  )
}

export default SampleItem
