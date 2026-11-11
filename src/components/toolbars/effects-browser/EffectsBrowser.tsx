import React from 'react'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import { fetchPlugins, scanPlugins, selectCatalog } from '../../../state/slices/effectsSlice'
import { beginSampleDrag } from '../../../state/sampleDrag'
import { FailedPlugin, groupPlugins, PluginInfo, PluginKind } from '../../../types/effects'
import { isMac } from '../../../util/shortcuts'
import { AnalogButton, ResetIcon } from '../../fourtrack'
import { Bar } from '../../common'
import BrowserSearch from '../browser/BrowserSearch'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import { Chevron, FolderIcon, KnobIcon, WarningIcon } from '../browser/icons'
import { useSessionSet } from '../browser/useSessionSet'

// how often the engine is asked how a scan is going
export const POLL_MS = 300

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// where VST3 plugins are installed, on this system (where the engine looks)
const pluginFolders = (): string[] => {
  if(isMac()) return ['/Library/Audio/Plug-Ins/VST3', '~/Library/Audio/Plug-Ins/VST3']
  if(typeof navigator !== 'undefined' && /win/i.test(navigator.platform)) {
    return ['C:\\Program Files\\Common Files\\VST3']
  }
  return ['~/.vst3', '/usr/lib/vst3', '/usr/local/lib/vst3']
}

const fileName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path

// the kinds of plugin the browser lists, each in a section of its own
// (instruments join when they can be played)
const SECTIONS: Array<{ kind: PluginKind, plural: string }> = [
  { kind: 'effect', plural: 'effects' },
]

// one plugin: drag it onto a mixer channel, or into the Effects Rack
const PluginRow = ({ plugin }: { plugin: PluginInfo }) => (
  <BrowserRow
    role='treeitem'
    selected={false}
    title={`${plugin.name} by ${plugin.vendor}${plugin.version ? ` (${plugin.version})` : ''}. Drag onto a mixer channel or into the Effects Rack`}
    style={{ paddingLeft: 20 }}
    onMouseDown={ev => {
      if(ev.button !== 0) return
      beginSampleDrag({ kind: 'effect', pluginId: plugin.id, label: plugin.name }, ev)
    }}
  >
    <KnobIcon />
    <span className='truncate'>{plugin.name}</span>
    <RowHint className='ml-auto pl-2 max-w-[45%]'>{plugin.vendor}</RowHint>
  </BrowserRow>
)

// a category's folder: click to open or close it
const CategoryRow = ({ category, count, open, onToggle }: {
  category: string
  count: number
  open: boolean
  onToggle: () => void
}) => (
  <BrowserRow
    role='treeitem'
    aria-expanded={open}
    selected={false}
    grab={false}
    title={category}
    onClick={onToggle}
  >
    <Chevron open={open} />
    <FolderIcon />
    <span className='truncate font-medium'>{category}</span>
    <RowHint className='ml-auto pl-2'>{count}</RowHint>
  </BrowserRow>
)

// the bundles that couldn't be read, and why, closed until asked for
const FailedList = ({ failed }: { failed: FailedPlugin[] }) => {
  const [open, setOpen] = React.useState(false)

  return (
    <div className='border-t border-line mt-1 pt-1'>
      <BrowserRow
        aria-expanded={open}
        selected={false}
        grab={false}
        title='Plugins that crashed, hung or couldn’t be read while scanning. Refresh tries them again'
        onClick={() => setOpen(!open)}
      >
        <Chevron open={open} />
        <WarningIcon />
        <span className='truncate'>Couldn’t be read ({failed.length})</span>
      </BrowserRow>
      {open && failed.map(item => (
        <div key={item.path} className='pl-7 pr-3 py-1 text-xs' title={item.path}>
          <div className='truncate text-ink-mid'>{fileName(item.path)}</div>
          <div className='text-[10px] text-ink-faint break-words'>{item.reason}</div>
        </div>
      ))}
    </div>
  )
}

// a kind's plugins in folders by category. searching opens every folder
// with a match
const PluginSection = ({ plugins, kind, query }: { plugins: PluginInfo[], kind: PluginKind, query: string }) => {
  const [expanded, setExpanded] = useSessionSet(`plugins-${kind}`)
  const groups = groupPlugins(plugins, kind, query)
  const searching = !!query.trim()

  const toggle = (category: string) => setExpanded(prev => {
    const next = new Set(prev)
    if(next.has(category)) next.delete(category)
    else next.add(category)
    return next
  })

  return (
    <>
      {groups.map(({ category, plugins: inCategory }) => {
        const open = searching || expanded.has(category)
        return (
          <React.Fragment key={category}>
            <CategoryRow
              category={category}
              count={inCategory.length}
              open={open}
              onToggle={() => toggle(category)}
            />
            {open && inCategory.map(plugin => <PluginRow key={plugin.id} plugin={plugin} />)}
          </React.Fragment>
        )
      })}
    </>
  )
}

// the effect plugins the engine found, by category: drag one onto a mixer
// channel (or into the Effects Rack) to use it
const EffectsBrowser = () => {
  const dispatch = useAppDispatch()
  const catalog = useAppSelector(selectCatalog)
  const [query, setQuery] = React.useState('')
  const scanning = !!catalog?.scanning

  // what's been found so far. the engine looks for new plugins by itself
  // at launch, so a scan may be going already
  React.useEffect(() => {
    dispatch(fetchPlugins()).catch(logError('list the plugins'))
  }, [dispatch])

  // and how a scan is going, while it goes
  React.useEffect(() => {
    if(!scanning) return undefined
    const timer = setInterval(() => {
      dispatch(fetchPlugins()).catch(logError('list the plugins'))
    }, POLL_MS)
    return () => clearInterval(timer)
  }, [dispatch, scanning])

  const rescan = () => {
    dispatch(scanPlugins(true)).catch(logError('scan for plugins'))
  }

  const plugins = catalog?.plugins ?? []
  const failed = catalog?.failed ?? []
  const searching = !!query.trim()

  return (
    <div className='flex flex-col h-full min-h-0'>
      <BrowserSearch value={query} onChange={setQuery} label='Search effects'>
        <AnalogButton
          size='xs'
          aria-label='Rescan plugins'
          title={scanning ? 'Scanning for plugins…' : 'Look for plugins again, including those that couldn’t be read'}
          disabled={scanning}
          onPress={rescan}
        >
          <ResetIcon />
        </AnalogButton>
      </BrowserSearch>

      {scanning && catalog && (
        <div className='px-3 pb-2 flex flex-col gap-1' role='status'>
          <span className='text-xs text-ink-soft'>
            {catalog.total > 0 ? `Scanning ${catalog.done} of ${catalog.total}…` : 'Scanning…'}
          </span>
          <Bar
            value={catalog.total > 0 ? Math.min(1, catalog.done / catalog.total) : 0}
            className='w-full'
            role='progressbar'
            aria-valuemin={0}
            aria-valuemax={catalog.total}
            aria-valuenow={catalog.done}
          />
        </div>
      )}

      <div className='flex-1 min-h-0 overflow-y-auto overflow-x-hidden pb-2' role='tree' aria-label='Effects'>
        {SECTIONS.map(({ kind, plural }) => {
          const ofKind = plugins.filter(plugin => plugin.kind === kind)
          const matching = groupPlugins(plugins, kind, query).length > 0

          if(!ofKind.length) {
            if(!catalog || scanning) {
              return (
                <div key={kind} className='px-3 py-2 text-xs text-ink-faint'>
                  {catalog ? 'Looking for plugins…' : 'Loading…'}
                </div>
              )
            }
            return (
              <div key={kind} className='px-3 py-2 text-xs text-ink-faint'>
                <p className='text-ink-soft'>No VST3 {plural} found</p>
                <p className='mt-1'>Looked in:</p>
                <ul className='mt-0.5'>
                  {pluginFolders().map(folder => (
                    <li key={folder} className='font-mono text-[10px] break-all'>{folder}</li>
                  ))}
                </ul>
              </div>
            )
          }

          if(searching && !matching) {
            return <div key={kind} className='px-3 py-2 text-xs text-ink-faint'>No {plural} match.</div>
          }

          return <PluginSection key={kind} plugins={plugins} kind={kind} query={query} />
        })}

        {failed.length > 0 && <FailedList failed={failed} />}
      </div>
    </div>
  )
}

export default EffectsBrowser
