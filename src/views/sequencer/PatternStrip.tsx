import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern,
  duplicatePattern,
  removePattern,
  renamePattern,
  selectPattern,
  selectPatterns,
  selectSelectedPattern,
  setPatternBars,
} from '../../state/slices/sequencerSlice'
import { selectPlaylistItems } from '../../state/slices/playlistSlice'
import { beginSampleDrag } from '../../state/sampleDrag'
import { BAR_CHOICES, Pattern, patternBars, patternLabel } from '../../types/sequencer'
import { AnalogButton, Led } from '../../components/fourtrack'

const MAX_NAME_CHARS = 24

// a pattern's key: click to show it, double-click to name it, drag it into
// the playlist to place it there
const PatternKey = ({ pattern, selected, playing }: {
  pattern: Pattern
  selected: boolean
  playing: boolean
}) => {
  const dispatch = useAppDispatch()
  const [draft, setDraft] = React.useState<string | null>(null)
  const label = patternLabel(pattern)

  if(draft !== null) {
    const commit = () => {
      dispatch(renamePattern(pattern.id, draft))
      setDraft(null)
    }

    return (
      <input
        type='text'
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        aria-label='Pattern name'
        placeholder={String(pattern.id + 1)}
        maxLength={MAX_NAME_CHARS}
        className='ft-scribble h-7 w-28 text-xs text-center px-1 outline-none ring-2 ring-sky-400'
        value={draft}
        onFocus={e => e.currentTarget.select()}
        onChange={e => setDraft(e.currentTarget.value)}
        onBlur={commit}
        onKeyDown={e => {
          if(e.key === 'Enter') commit()
          if(e.key === 'Escape') setDraft(null)
        }}
      />
    )
  }

  return (
    <div className='flex flex-col items-center gap-1'>
      <Led color='lime' state={playing ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='sm'
        latched={selected}
        className='min-w-[2.5rem] max-w-[9rem] truncate px-2 normal-case'
        title={`${label}. Drag into the playlist to place it, double-click to rename`}
        onMouseDown={ev => beginSampleDrag({ kind: 'pattern', patternId: pattern.id, label }, ev)}
        onPress={() => dispatch(selectPattern(pattern.id))}
        onDoubleClick={() => setDraft(pattern.name ?? '')}
      >
        {label}
      </AnalogButton>
    </div>
  )
}

// deletes the pattern (and its clips in the playlist), after a second click
const DeletePattern = ({ pattern }: { pattern: Pattern }) => {
  const dispatch = useAppDispatch()
  const [asking, setAsking] = React.useState(false)
  const clips = useAppSelector(selectPlaylistItems)
    .filter(item => item.patternId === pattern.id).length

  // a different pattern selected starts over
  React.useEffect(() => { setAsking(false) }, [pattern.id])

  // stop asking after a while, so a stray click later doesn't delete
  React.useEffect(() => {
    if(!asking) return undefined
    const timer = setTimeout(() => setAsking(false), 3000)
    return () => clearTimeout(timer)
  }, [asking])

  let sure = 'Sure?'
  if(clips === 1) sure = 'And its clip?'
  else if(clips > 1) sure = `And ${clips} clips?`

  return (
    <AnalogButton
      variant='text'
      size='sm'
      className={asking ? 'ft-btn--danger' : ''}
      title={clips
        ? `Delete the pattern, and its ${clips === 1 ? 'clip' : `${clips} clips`} in the playlist`
        : 'Delete the pattern'}
      onPress={() => {
        if(asking) {
          setAsking(false)
          dispatch(removePattern(pattern.id))
        } else {
          setAsking(true)
        }
      }}
    >
      {asking ? sure : 'Delete'}
    </AnalogButton>
  )
}

// the patterns along the top, and the selected one's length and keys
const PatternStrip = ({ beatsPerBar, playingId }: { beatsPerBar: number, playingId: number | null }) => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatterns)
  const selected = useAppSelector(selectSelectedPattern)
  const bars = selected ? patternBars(selected, beatsPerBar) : null

  return (
    <div className='ft-tray rounded-lg m-2 mb-0 p-2 flex flex-row flex-wrap items-end gap-x-4 gap-y-2'>
      <div className='flex flex-col gap-1 min-w-0'>
        <span className='ft-label'>Patterns</span>
        <div className='flex flex-row flex-wrap items-end gap-1.5'>
          {patterns.map(pattern => (
            <PatternKey
              key={pattern.id}
              pattern={pattern}
              selected={pattern.id === selected?.id}
              playing={pattern.id === playingId}
            />
          ))}
          <AnalogButton
            size='sm'
            variant='text'
            title='A new, empty pattern'
            aria-label='New pattern'
            onPress={() => dispatch(addPattern(bars ?? 1))}
          >
            +
          </AnalogButton>
        </div>
      </div>

      {selected && (
        <>
          <div className='flex flex-col gap-1'>
            <span className='ft-label'>Bars</span>
            <div className='flex flex-row gap-1'>
              {BAR_CHOICES.map(n => (
                <AnalogButton
                  key={n}
                  variant='text'
                  size='sm'
                  latched={bars === n}
                  title={`Make the pattern ${n} bar${n === 1 ? '' : 's'} long`}
                  onPress={() => dispatch(setPatternBars(selected.id, n))}
                >
                  {n}
                </AnalogButton>
              ))}
            </div>
          </div>

          <div className='flex flex-row gap-1 ml-auto'>
            <AnalogButton
              variant='text'
              size='sm'
              title='A copy of this pattern, to make a variation of it'
              onPress={() => dispatch(duplicatePattern(selected.id))}
            >
              Copy
            </AnalogButton>
            <DeletePattern pattern={selected} />
          </div>
        </>
      )}
    </div>
  )
}

export default PatternStrip
