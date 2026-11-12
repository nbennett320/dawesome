import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  baseName,
  getBrowserRootSamples,
  getDirectorySamples,
  selectBrowserSamples,
} from '../../../state/slices/envSlice'
import { useAppSelector, useAppDispatch } from '../../../hooks/redux'
import { BrowserItemTypes, BrowserSampleItem } from '../../../types/sampleBrowser'
import { AnalogButton } from '../../fourtrack'
import SampleItem from './SampleItem'
import BrowserSearch from '../browser/BrowserSearch'
import { ResetIcon } from '../../icons'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// the samples folder as a tree. click a folder to open it, click a sample to
// hear it, drag a sample into the playlist to add it
const SampleBrowser = () => {
  const items = useAppSelector(selectBrowserSamples)
  const dispatch = useAppDispatch()
  const [expanded, setExpanded] = React.useState<Set<string>>(new Set())
  // folders listed already, so opening them again doesn't ask the engine
  const [listed, setListed] = React.useState<Set<string>>(new Set())
  // the sample heard last
  const [selected, setSelected] = React.useState<string | null>(null)
  const [query, setQuery] = React.useState('')

  React.useEffect(() => {
    dispatch(getBrowserRootSamples()).catch(logError('list the samples folder'))
  }, [dispatch])

  // const refresh = () => {
  //   setListed(new Set())
  //   dispatch(getBrowserRootSamples()).catch(logError('list the samples folder'))
  //   expanded.forEach(path => {
  //     dispatch(getDirectorySamples(path)).catch(logError(`list ${path}`))
  //   })
  // }

  const toggleFolder = (folder: BrowserSampleItem) => {
    const next = new Set(expanded)
    if(next.has(folder.index)) {
      next.delete(folder.index)
    } else {
      next.add(folder.index)
      if(!listed.has(folder.index)) {
        setListed(new Set(listed).add(folder.index))
        dispatch(getDirectorySamples(folder.path)).catch(logError(`list ${folder.path}`))
      }
    }
    setExpanded(next)
  }

  const preview = (sample: BrowserSampleItem) => {
    setSelected(sample.index)
    invoke('t_preview_sample', { path: sample.path }).catch(logError(`preview ${sample.label}`))
  }

  const rows: React.ReactNode[] = []
  const addRows = (parent: BrowserSampleItem, depth: number) => {
    parent.children.forEach(key => {
      const item = items[key]
      if(!item) return

      const open = expanded.has(item.index)
      rows.push(
        <SampleItem
          key={item.index}
          item={item}
          depth={depth}
          open={open}
          selected={selected === item.index}
          onToggle={toggleFolder}
          onPreview={preview}
        />,
      )

      if(item.itemType === BrowserItemTypes.Directory && open) {
        if(item.children.length) addRows(item, depth + 1)
        else {
          rows.push(
            <div key={`${item.index}/empty`} className='ft-label normal-case tracking-normal italic h-6 leading-6' style={{ paddingLeft: (depth + 1) * 12 + 26 }}>
              {listed.has(item.index) ? 'Empty' : 'Loading…'}
            </div>,
          )
        }
      }
    })
  }

  const search = query.trim().toLowerCase()
  if(search) {
    // the samples in folders listed so far, by name
    Object.values(items)
      .filter(item => item.itemType === BrowserItemTypes.Sample && item.label.toLowerCase().includes(search))
      .sort((a, b) => a.label.localeCompare(b.label, undefined, { numeric: true, sensitivity: 'base' }))
      .forEach(item => {
        const folder = baseName(item.path.slice(0, item.path.length - baseName(item.path).length))
        rows.push(
          <SampleItem
            key={item.index}
            item={item}
            depth={0}
            open={false}
            selected={selected === item.index}
            hint={folder}
            onToggle={toggleFolder}
            onPreview={preview}
          />,
        )
      })
  } else if(items.root) {
    addRows(items.root, 0)
  }

  return (
    <div className='flex flex-col h-full min-h-0'>
      {/* <div className='flex flex-row items-center justify-between px-3 pt-3 pb-2'>
        <AnalogButton size='xs' aria-label='Refresh samples' title='List the samples folder again' onPress={refresh}>
          <ResetIcon />
        </AnalogButton>
      </div> */}

      <BrowserSearch value={query} onChange={setQuery} label='Search samples' />

      <div className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden pb-2' role='tree' aria-label='Samples'>
        {rows}
        {search && !rows.length && (
          <div className='px-3 py-2 text-xs text-ink-faint'>
            No samples match. Only folders opened so far are searched.
          </div>
        )}
      </div>
    </div>
  )
}

export default SampleBrowser
