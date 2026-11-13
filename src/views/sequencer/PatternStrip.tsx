import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addPattern,
  duplicatePattern,
  removePattern,
  renamePattern,
  selectPattern,
  selectPatternsOf,
  selectSelectedPattern,
  setPatternBars,
} from '../../state/slices/sequencerSlice'
import { selectPlaylistItems } from '../../state/slices/playlistSlice'
import { beginSampleDrag } from '../../state/sampleDrag'
import {
  BAR_CHOICES, MAX_NOTE_BARS, Pattern, PatternKind, patternBars, patternLabel, unnamedLabel,
} from '../../types/sequencer'
import { AnalogButton, Led } from '../../components/fourtrack'
import { NameInput } from '../../components/common'
import { PatternMenu, ScrollingKeys } from './PatternKeys'

// narrower than this, the selected pattern's length and keys go on a line
// of their own under the patterns
const NARROW_PX = 560

const MAX_NAME_CHARS = 24

// a pattern's key: click to show it, double-click to name it, drag it into
// the playlist to place it there
const PatternKey = ({ pattern, selected, playing }: {
  pattern: Pattern
  selected: boolean
  playing: boolean
}) => {
  const dispatch = useAppDispatch()
  const [renaming, setRenaming] = React.useState(false)
  const label = patternLabel(pattern)

  if(renaming) {
    // the size of the key it stands in for: as tall, and about as wide as
    // the name (the light above stays, so nothing moves)
    const chars = Math.min(Math.max(label.length, 6), MAX_NAME_CHARS) + 2
    return (
      <div className='shrink-0 flex flex-col items-center gap-1' data-pattern-id={pattern.id}>
        <Led color='lime' state={playing ? 'on' : 'off'} />
        <NameInput
          initial={pattern.name ?? ''}
          aria-label='Pattern name'
          placeholder={unnamedLabel(pattern)}
          maxLength={MAX_NAME_CHARS}
          className='h-[22px] text-[10px] text-center rounded'
          style={{ width: `${chars}ch` }}
          onDone={name => {
            if(name !== null) dispatch(renamePattern(pattern.id, name))
            setRenaming(false)
          }}
        />
      </div>
    )
  }

  return (
    <div className='shrink-0 flex flex-col items-center gap-1' data-pattern-id={pattern.id}>
      <Led color='lime' state={playing ? 'on' : 'off'} />
      <AnalogButton
        variant='text'
        size='sm'
        latched={selected}
        className='min-w-[2.5rem] max-w-[9rem] truncate px-2 normal-case'
        title={`${label}. Drag into the playlist to place it, double-click to rename`}
        onMouseDown={ev => beginSampleDrag({ kind: 'pattern', patternId: pattern.id, label }, ev)}
        onPress={() => dispatch(selectPattern(pattern.id))}
        onDoubleClick={() => setRenaming(true)}
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

// a drum pattern's length: 1, 2 or 4 bars
const BarChoices = ({ pattern, bars }: { pattern: Pattern, bars: number | null }) => {
  const dispatch = useAppDispatch()

  return (
    <div className='flex flex-row gap-1'>
      {BAR_CHOICES.map(n => (
        <AnalogButton
          key={n}
          variant='text'
          size='sm'
          latched={bars === n}
          title={`Make the pattern ${n} bar${n === 1 ? '' : 's'} long`}
          onPress={() => dispatch(setPatternBars(pattern.id, n))}
        >
          {n}
        </AnalogButton>
      ))}
    </div>
  )
}

// whether a typed length is one a note pattern can be
const validBars = (bars: number) => Number.isInteger(bars) && bars >= 1 && bars <= MAX_NOTE_BARS

// a note pattern's length: any number of bars up to MAX_NOTE_BARS, typed.
// taken up on Enter or when the field is left
const BarsField = ({ pattern, bars }: { pattern: Pattern, bars: number | null }) => {
  const dispatch = useAppDispatch()
  const [draft, setDraft] = React.useState<string | null>(null)

  const takeUp = (value: string) => {
    setDraft(null)
    const typed = Number(value)
    if(validBars(typed) && typed !== bars) dispatch(setPatternBars(pattern.id, typed))
  }

  return (
    <input
      type='number'
      min={1}
      max={MAX_NOTE_BARS}
      step={1}
      className='ft-readout text-sm py-0.5 w-14 h-7'
      aria-label='Bars'
      title={`How many bars long the pattern is, 1 to ${MAX_NOTE_BARS}. Notes past its end are kept, but don’t play`}
      value={draft ?? (bars ?? '')}
      onChange={e => setDraft(e.currentTarget.value)}
      onBlur={e => takeUp(e.currentTarget.value)}
      onKeyDown={e => {
        if(e.key === 'Enter') takeUp(e.currentTarget.value)
        if(e.key === 'Escape') setDraft(null)
      }}
    />
  )
}

// whether `ref` is narrower than `px`, following it as it's resized
const useNarrow = (ref: React.RefObject<HTMLElement>, px: number): boolean => {
  const [narrow, setNarrow] = React.useState(false)
  React.useEffect(() => {
    const el = ref.current
    if(!el || typeof ResizeObserver === 'undefined') return undefined
    const observer = new ResizeObserver(() => setNarrow(el.clientWidth < px))
    observer.observe(el)
    return () => observer.disconnect()
  }, [ref, px])
  return narrow
}

// the patterns of a kind along the top, in one line that scrolls, then the
// selected one's length, Copy and Delete (under them, when the strip is
// narrow). a sequencer's own: the Drum Sequencer's drum patterns, or the
// Note Sequencer's note patterns. the top bar's Play loops the selected one
const PatternStrip = ({ kind, beatsPerBar, playingId }: {
  kind: PatternKind
  beatsPerBar: number
  playingId: number | null
}) => {
  const dispatch = useAppDispatch()
  const patterns = useAppSelector(selectPatternsOf(kind))
  const selected = useAppSelector(selectSelectedPattern(kind))
  const bars = selected ? patternBars(selected, beatsPerBar) : null
  const ref = React.useRef<HTMLDivElement>(null)
  const narrow = useNarrow(ref, NARROW_PX)

  const controls = selected && (
    <>
      <div className='shrink-0 flex flex-col gap-1'>
        <span className='ft-label'>Bars</span>
        {kind === 'drum'
          ? <BarChoices pattern={selected} bars={bars} />
          : <BarsField key={selected.id} pattern={selected} bars={bars} />}
      </div>

      <div className='shrink-0 flex flex-row gap-1 ml-auto'>
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
  )

  return (
    <div ref={ref} className='ft-tray rounded-lg m-2 mb-0 p-2 flex flex-col gap-2'>
      <div className='flex flex-row items-end gap-x-4'>
        <div className='flex-1 min-w-0 flex flex-col gap-1'>
          <span className='ft-label'>Patterns</span>
          <div className='flex flex-row items-end gap-1.5 min-w-0'>
            <ScrollingKeys selectedId={selected?.id ?? null}>
              {patterns.map(pattern => (
                <PatternKey
                  key={pattern.id}
                  pattern={pattern}
                  selected={pattern.id === selected?.id}
                  playing={pattern.id === playingId}
                />
              ))}
            </ScrollingKeys>
            {patterns.length > 0 && (
              <PatternMenu
                patterns={patterns}
                selectedId={selected?.id ?? null}
                playingId={playingId}
                onPick={id => dispatch(selectPattern(id))}
              />
            )}
            <div className='shrink-0 pb-1'>
              <AnalogButton
                size='sm'
                variant='text'
                title='A new, empty pattern'
                aria-label='New pattern'
                onPress={() => dispatch(addPattern(bars ?? 1, kind))}
              >
                +
              </AnalogButton>
            </div>
          </div>
        </div>
        {!narrow && controls}
      </div>
      {narrow && controls && <div className='flex flex-row items-end gap-4'>{controls}</div>}
    </div>
  )
}

// memo: the Drum Sequencer re-renders as the playing step moves, which
// doesn't change the strip
export default React.memo(PatternStrip)
