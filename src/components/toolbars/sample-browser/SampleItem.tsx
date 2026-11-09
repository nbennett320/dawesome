import React from 'react'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { BrowserItemTypes, BrowserSampleItem } from '../../../types/sampleBrowser'

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

const Chevron = ({ open }: { open: boolean }) => (
  <svg
    viewBox='0 0 16 16'
    className={`w-3 h-3 shrink-0 text-slate-400 transition-transform ${open ? 'rotate-90' : ''}`}
    aria-hidden
  >
    <path d='M6 4l4 4-4 4' fill='none' stroke='currentColor' strokeWidth='1.75' strokeLinecap='round' strokeLinejoin='round' />
  </svg>
)

const FolderIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5 shrink-0 text-slate-400' aria-hidden>
    <path d='M1.5 4.5a1 1 0 0 1 1-1h3.6l1.4 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z' fill='currentColor' opacity='.35' stroke='currentColor' strokeWidth='1' />
  </svg>
)

// a little waveform, for a sample
const WaveIcon = () => (
  <svg viewBox='0 0 16 16' className='w-3.5 h-3.5 shrink-0 text-slate-400' aria-hidden>
    <path d='M2 8h1.5M5 5v6M7.5 3v10M10 6v4M12.5 7v2M14 8h0' fill='none' stroke='currentColor' strokeWidth='1.5' strokeLinecap='round' />
  </svg>
)

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
    if(ev.key !== 'Enter' && ev.key !== ' ') return
    ev.preventDefault()
    if(isFolder) onToggle(item)
    else onPreview(item)
  }

  return (
    <button
      type='button'
      role='treeitem'
      aria-expanded={isFolder ? open : undefined}
      aria-selected={selected}
      title={item.path}
      className={`group w-full h-6 flex flex-row items-center gap-1.5 pr-3 text-left text-xs outline-none
        border-l-2 focus-visible:bg-sky-50 ${
          selected
            ? 'border-sky-500 bg-sky-50 text-slate-800'
            : 'border-transparent text-slate-600 hover:bg-slate-100 hover:text-slate-800'
        } ${isFolder ? 'cursor-pointer' : 'cursor-grab active:cursor-grabbing'}`}
      style={{ paddingLeft: depth * 12 + 8 }}
      onMouseDown={onMouseDown}
      onKeyDown={onKeyDown}
    >
      {isFolder ? <Chevron open={open} /> : <span className='w-3 shrink-0' />}
      {isFolder ? <FolderIcon /> : <WaveIcon />}
      <span className={`truncate ${isFolder ? 'font-medium' : ''}`}>{item.label}</span>
      {hint && <span className='ml-auto pl-2 shrink-0 text-[10px] text-slate-400 truncate max-w-[40%]'>{hint}</span>}
    </button>
  )
}

export default SampleItem
