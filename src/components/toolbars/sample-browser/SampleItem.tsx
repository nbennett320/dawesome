import React from 'react'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { BrowserItemTypes, BrowserSampleItem } from '../../../types/sampleBrowser'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import { ChevronIcon, FolderIcon, StopIcon, WaveIcon } from '../../icons'

interface Props {
  item: BrowserSampleItem
  depth: number
  // a folder that's open
  open: boolean
  // the sample heard last
  selected: boolean
  // the sample being heard, while it plays: it shows a stop button
  playing?: boolean
  // shown faintly after the name, e.g. the folder a search result is in
  hint?: string
  onToggle: (folder: BrowserSampleItem) => void
  onPreview: (sample: BrowserSampleItem) => void
  onStop?: () => void
}

// stops the preview, in the slot a folder's chevron takes. it's inside the
// row's button, so it's a span, and pressing it doesn't play or drag the
// sample again. from the keyboard, Escape on the row stops it instead (so
// it's not a tab stop of its own)
const StopPreview = ({ onStop }: { onStop: () => void }) => (
  <span
    role='button'
    tabIndex={-1}
    aria-label='Stop preview'
    title='Stop preview'
    className='w-3 h-3 shrink-0 flex items-center justify-center text-accent hover:text-ink-strong cursor-pointer'
    onMouseDown={ev => {
      ev.stopPropagation()
      ev.preventDefault()
      if(ev.button === 0) onStop()
    }}
  >
    <StopIcon className='w-3 h-3' />
  </span>
)

// one row of the sample browser: a folder, or a sample to hear and drag
const SampleItem = ({ item, depth, open, selected, playing = false, hint, onToggle, onPreview, onStop }: Props) => {
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
    if(ev.key === 'Escape' && playing && onStop) {
      // (and nothing else listening for Escape hears it)
      ev.preventDefault()
      ev.stopPropagation()
      onStop()
      return
    }
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
      {isFolder && <ChevronIcon open={open} />}
      {!isFolder && (playing && onStop ? <StopPreview onStop={onStop} /> : <span className='w-3 shrink-0' />)}
      {isFolder ? <FolderIcon /> : <WaveIcon />}
      <span className={`truncate ${isFolder ? 'font-medium' : ''}`}>{item.label}</span>
      {hint && <RowHint className='ml-auto pl-2 max-w-[40%]'>{hint}</RowHint>}
    </BrowserRow>
  )
}

export default SampleItem
