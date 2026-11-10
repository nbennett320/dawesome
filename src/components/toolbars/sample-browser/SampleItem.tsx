import React from 'react'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { BrowserItemTypes, BrowserSampleItem } from '../../../types/sampleBrowser'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import { Chevron, FolderIcon, WaveIcon } from '../browser/icons'

interface Props {
  item: BrowserSampleItem
  depth: number
  // a folder that's open
  open: boolean
  // the sample heard last
  selected: boolean
  // shown faintly after the name, e.g. the folder a search result is in
  hint?: string
  onToggle: (folder: BrowserSampleItem) => void
  onPreview: (sample: BrowserSampleItem) => void
}

// one row of the sample browser: a folder, or a sample to hear and drag
const SampleItem = ({ item, depth, open, selected, hint, onToggle, onPreview }: Props) => {
  const isFolder = item.itemType === BrowserItemTypes.Directory

  const onMouseDown = (ev: React.MouseEvent) => {
    if(ev.button !== 0) return
    if(isFolder) {
      onToggle(item)
    } else {
      onPreview(item)
      // dragging the sample into the playlist (or the sequencer) adds it there
      beginSampleDrag({ kind: 'sample', path: item.path }, ev)
    }
  }

  const onKeyDown = (ev: React.KeyboardEvent) => {
    // (space plays and pauses, see App)
    if(ev.key !== 'Enter') return
    ev.preventDefault()
    if(isFolder) onToggle(item)
    else onPreview(item)
  }

  return (
    <BrowserRow
      role='treeitem'
      aria-expanded={isFolder ? open : undefined}
      selected={selected}
      grab={!isFolder}
      title={item.path}
      style={{ paddingLeft: depth * 12 + 8 }}
      onMouseDown={onMouseDown}
      onKeyDown={onKeyDown}
    >
      {isFolder ? <Chevron open={open} /> : <span className='w-3 shrink-0' />}
      {isFolder ? <FolderIcon /> : <WaveIcon />}
      <span className={`truncate ${isFolder ? 'font-medium' : ''}`}>{item.label}</span>
      {hint && <RowHint className='ml-auto pl-2 max-w-[40%]'>{hint}</RowHint>}
    </BrowserRow>
  )
}

export default SampleItem
