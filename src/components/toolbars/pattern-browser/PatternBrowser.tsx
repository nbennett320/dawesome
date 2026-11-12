import React from 'react'
import { useAppDispatch, useAppSelector } from '../../../hooks/redux'
import {
  addPatternFolder,
  loadSequencer,
  movePatternFolder,
  movePatternToFolder,
  removePatternFolder,
  renamePatternFolder,
  selectFoldersOf,
  selectPattern,
  selectPatternRun,
  selectPatternsOf,
  selectSelectedPatternId,
  sequencerView,
} from '../../../state/slices/sequencerSlice'
import { selectPlaylistItems } from '../../../state/slices/playlistSlice'
import { openTab } from '../../../state/slices/windowSlice'
import { FolderScope, beginSampleDrag } from '../../../state/sampleDrag'
import { FolderNode, allItems, buildTree } from '../../../types/folderTree'
import {
  Pattern, PatternKind, folderIdOf, folderKey, folderPathLabel, patternFolderInfos, patternHint, patternLabel,
  searchPatterns,
} from '../../../types/sequencer'
import { viewLabels } from '../../../types/ui'
import { plural } from '../../../util/helpers'
import { useTransport } from '../../../views/transport/useTransport'
import { Led } from '../../fourtrack'
import BrowserRow, { RowHint } from '../browser/BrowserRow'
import FolderBrowser, { ItemContext } from '../browser/FolderBrowser'
import { useClickWithoutDrag } from '../browser/useClickWithoutDrag'

// each kind's page: its folders' drag scope, and what it's called
const PAGES: Record<PatternKind, { scope: FolderScope, label: string }> = {
  drum: { scope: 'patterns', label: 'Drum patterns' },
  note: { scope: 'note-patterns', label: 'Note patterns' },
}

interface RowProps {
  pattern: Pattern
  // the sequencer it's shown in
  shownIn: string
  hint: string
  // placed in the playlist at least once
  used: boolean
  selected: boolean
  playing: boolean
  depth: number
  onSelect: (pattern: Pattern) => void
}

// one pattern: click to show it in its sequencer, drag it into the
// playlist or a folder
const PatternRow = ({ pattern, shownIn, hint, used, selected, playing, depth, onSelect }: RowProps) => {
  const label = patternLabel(pattern)
  const { pressed, isClick } = useClickWithoutDrag()

  return (
    <BrowserRow
      role='treeitem'
      selected={selected}
      used={used}
      title={`${label}. Click to show it in the ${shownIn}, drag into the playlist to place it`}
      style={{ paddingLeft: depth * 12 + 8 }}
      // only a drag starts on mousedown: showing the sequencer straight away
      // could hide the playlist (when they share a pane) before it's dropped
      onMouseDown={ev => {
        if(ev.button !== 0) return
        pressed(ev)
        beginSampleDrag({ kind: 'pattern', patternId: pattern.id, label }, ev)
      }}
      // also enter and space, which buttons turn into clicks
      onClick={ev => { if(isClick(ev)) onSelect(pattern) }}
    >
      <Led color='lime' state={playing ? 'on' : 'off'} className='shrink-0 scale-75' />
      <span className='truncate'>{label}</span>
      <RowHint className='ml-auto pl-2'>{hint}</RowHint>
    </BrowserRow>
  )
}

// a kind's patterns (the Drum Sequencer's, or the Note Sequencer's), in
// folders of that kind: find them by name, drag them into the playlist, and
// file them away
const PatternBrowser = ({ kind }: { kind: PatternKind }) => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatternsOf(kind))
  const folders = useAppSelector(selectFoldersOf(kind))
  const selectedId = useAppSelector(selectSelectedPatternId(kind))
  const run = useAppSelector(selectPatternRun)
  const items = useAppSelector(selectPlaylistItems)
  const { transport } = useTransport()
  const beatsPerBar = transport?.beatsPerBar ?? 4
  const { scope, label } = PAGES[kind]
  const shownIn = viewLabels[sequencerView(kind)]

  // the engine owns the sequencer: catch up with it, in case its tab hasn't
  // been opened yet
  React.useEffect(() => { dispatch(loadSequencer()) }, [dispatch])

  const clipCounts = React.useMemo(() => {
    const counts = new Map<number, number>()
    items.forEach(item => {
      if(item.patternId !== undefined) counts.set(item.patternId, (counts.get(item.patternId) ?? 0) + 1)
    })
    return counts
  }, [items])

  const infos = React.useMemo(() => patternFolderInfos(folders), [folders])
  const tree = React.useMemo(() => buildTree(infos, patterns, p => folderKey(p.folder)), [infos, patterns])

  const placedIn = (node: FolderNode<Pattern>) =>
    allItems(node).filter(p => (clipCounts.get(p.id) ?? 0) > 0).length

  // an item row (and the folders') ids, as the engine knows them
  const withId = async (key: string, then: (id: number) => unknown) => {
    const id = folderIdOf(key)
    if(id !== null) await then(id)
  }

  const renderPattern = (pattern: Pattern, depth: number, { where, deselectFolder }: ItemContext) => {
    const clips = clipCounts.get(pattern.id) ?? 0
    const hint = patternHint(pattern, beatsPerBar, clips)
    return (
      <PatternRow
        pattern={pattern}
        shownIn={shownIn}
        hint={where ? `${where} · ${hint}` : hint}
        used={clips > 0}
        selected={pattern.id === selectedId}
        playing={pattern.id === run?.patternId}
        depth={depth}
        onSelect={() => {
          deselectFolder()
          dispatch(selectPattern(pattern.id))
          dispatch(openTab(sequencerView(kind)))
        }}
      />
    )
  }

  return (
    <FolderBrowser
      scope={scope}
      label={label}
      searchLabel='Search patterns'
      noun='pattern'
      folders={infos}
      tree={tree}
      itemKey={pattern => String(pattern.id)}
      renderItem={renderPattern}
      search={query => searchPatterns(patterns, query)
        .map(pattern => ({ item: pattern, where: folderPathLabel(folders, pattern.folder) }))}
      noMatch='No patterns match.'
      emptyMessage={!patterns.length && !folders.length ? `No patterns yet. Make one in the ${shownIn}.` : null}
      createFolder={(parent, name) => dispatch(addPatternFolder(name, folderIdOf(parent), kind)).then(folderKey)}
      renameFolder={(key, name) => withId(key, id => dispatch(renamePatternFolder(id, name)))}
      moveFolder={(key, into) => withId(key, id => dispatch(movePatternFolder(id, folderIdOf(into))))}
      deleteFolder={node => withId(node.key, id => dispatch(removePatternFolder(id)))}
      deleteLabel='Delete'
      deletePrompt={(name, what) => `Delete "${name}"${what}? This can't be undone.`}
      blockDelete={node => {
        const placed = placedIn(node)
        return placed
          ? `${plural(placed, 'pattern')} in it ${placed === 1 ? 'is' : 'are'} in the playlist. Remove those clips first`
          : undefined
      }}
      canDropItem={(item, key) => {
        if(item.kind !== 'pattern') return false
        const pattern = patterns.find(p => p.id === item.patternId)
        return !!pattern && folderKey(pattern.folder) !== key
      }}
      dropItem={async (item, key) => {
        if(item.kind === 'pattern') await dispatch(movePatternToFolder(item.patternId, folderIdOf(key)))
      }}
      dropFailure='move the pattern'
    />
  )
}

export default PatternBrowser
