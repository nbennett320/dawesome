import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  pausePlaylist,
  playPlaylist,
  seekPlaylist,
  setWind as setWindAction,
  stopPlaylist,
} from '../../state/slices/playlistSlice'
import {
  selectCassetteVisible,
  selectTapePathVisible,
  setCassetteVisible,
  setTapePathVisible,
} from '../../state/slices/windowSlice'
import { nameProject, projectName, selectProjectPath } from '../../state/slices/projectSlice'
import { AnalogButton, Cassette, cassetteAspect, Led, LedState } from '../../components/fourtrack'
import LatchKey from '../../components/topbar/LatchKey'
import {
  fitCassetteWidth,
  tapeLengthMs,
  tapTarget,
  Wind,
  windDone,
  windPosition,
  WIND_SPEED,
} from './tape'
import { usePlayhead } from './usePlayhead'
import { useElementSize } from './useElementSize'
import { FastForwardIcon, HeadIcon, PauseIcon, PlayIcon, RecordIcon, RewindIcon, StopIcon, TapeIcon } from '../../components/icons'

// holding a wind key this long winds the tape; a shorter press moves a bar
const HOLD_MS = 250

// the deck's padding, and the gap between the cassette and the keys (p-4,
// gap-5)
const PADDING = 16
const GAP = 20

// a key with its name printed under it, and room for an LED above
const DeckKey = ({ label, led, children }: { label: string, led?: React.ReactNode, children: React.ReactNode }) => (
  <div className='flex flex-col items-center gap-1'>
    {led ?? <span className='h-2' aria-hidden />}
    {children}
    <span className='ft-label text-[9px]'>{label}</span>
  </div>
)

// a cassette and the keys of a tape deck: record, rewind, fast forward, play,
// stop and pause. the wind keys move a bar when tapped, and wind the tape
// while held
const TransportDeck = () => {
  const dispatch = useAppDispatch()
  const cassetteVisible = useAppSelector(selectCassetteVisible)
  const tapePathVisible = useAppSelector(selectTapePathVisible)
  const label = projectName(useAppSelector(selectProjectPath)) ?? 'untitled'
  const {
    transport, position, wind, playing, refresh, assume,
  } = usePlayhead()
  const setWind = (next: Wind | null) => dispatch(setWindAction(next))
  // recording isn't connected to the engine yet, so arming only lights the key
  const [armed, setArmed] = React.useState(false)

  const tape = transport ? tapeLengthMs(transport) : 1
  const progress = Math.min(1, position / tape)

  // read from timers and key events, which outlive the render they're from
  const latest = React.useRef({ position, wind, playing })
  latest.current = { position, wind, playing }

  // the wind key being held: which way, whether playback was going (to carry
  // on after), and the timer that turns a tap into a wind
  const hold = React.useRef<{
    direction: 1 | -1,
    wasPlaying: boolean,
    timer: ReturnType<typeof setTimeout> | null,
  } | null>(null)

  const press = (direction: 1 | -1) => {
    if(!transport || hold.current) return

    const held = { direction, wasPlaying: latest.current.playing, timer: null as ReturnType<typeof setTimeout> | null }
    held.timer = setTimeout(() => {
      held.timer = null
      // winding is silent: playback pauses, and carries on from wherever the
      // tape is left
      setWind({ direction, fromMs: latest.current.position, startedAt: performance.now() })
      if(held.wasPlaying) dispatch(pausePlaylist())
    }, HOLD_MS)
    hold.current = held
  }

  const release = () => {
    const held = hold.current
    if(!held || !transport) return
    hold.current = null

    const tapped = held.timer !== null
    let target: number
    if(held.timer !== null) {
      clearTimeout(held.timer)
      target = tapTarget(latest.current.position, held.direction, transport)
    } else {
      const current = latest.current.wind
      target = current ? windPosition(current, performance.now(), transport) : latest.current.position
      setWind(null)
    }

    if(!latest.current.playing) assume(target)
    // a tap while playing carries on playing from the new bar by itself
    dispatch(seekPlaylist(target / transport.beatIntervalMs))
      .then(() => (!tapped && held.wasPlaying ? dispatch(playPlaylist()) : undefined))
      .then(refresh)
  }

  // in case the component goes while a key's held
  React.useEffect(() => () => {
    if(hold.current?.timer) clearTimeout(hold.current.timer)
  }, [])

  const windKeyProps = (direction: 1 | -1) => ({
    onPointerDown: (e: React.PointerEvent<HTMLButtonElement>) => {
      if(e.button !== 0) return
      e.currentTarget.setPointerCapture(e.pointerId)
      press(direction)
    },
    onPointerUp: release,
    onPointerCancel: release,
    onKeyDown: (e: React.KeyboardEvent) => {
      // (space plays and pauses, see App)
      if(e.key === 'Enter' && !e.repeat) {
        e.preventDefault()
        press(direction)
      }
    },
    onKeyUp: (e: React.KeyboardEvent) => {
      if(e.key === 'Enter') release()
    },
    onBlur: release,
  })

  const paused = !playing && !wind && position > 0

  // the reels: still at the end of the tape they're winding to
  let speed = 0
  if(wind && transport) {
    speed = windDone(wind, position, transport) ? 0 : wind.direction * WIND_SPEED
  } else if(playing && progress < 1) {
    speed = 1
  }

  // armed: blinks while waiting, lit while the tape runs
  let recordLed: LedState = 'off'
  if(armed) recordLed = playing ? 'on' : 'blink'

  // the cassette fills the room the keys leave, at its own shape
  const box = React.useRef<HTMLDivElement>(null)
  const controls = React.useRef<HTMLDivElement>(null)
  const room = useElementSize(box)
  const controlsSize = useElementSize(controls)
  const cassetteWidth = room && controlsSize
    ? fitCassetteWidth(
      room.width - PADDING * 2,
      room.height - PADDING * 2 - controlsSize.height - GAP,
      cassetteAspect(tapePathVisible),
    )
    : undefined


  return (
    // the tab's whole body, scrolling only when even the smallest cassette
    // and the keys don't fit
    <div ref={box} className='ft-panel h-full w-full overflow-auto'>
      <div className='min-h-full flex flex-col items-center justify-center gap-5 p-4'>
        {/* hidden until measured, so it doesn't start at the wrong size */}
        {cassetteVisible && (
          <div className='shrink-0 max-w-full' style={{ width: cassetteWidth, visibility: cassetteWidth ? undefined : 'hidden' }}>
            <Cassette
              progress={progress}
              speed={speed}
              label={label}
              tapePath={tapePathVisible}
              onRename={name => dispatch(nameProject(name))}
            />
          </div>
        )}

        <div ref={controls} className='flex flex-row flex-wrap items-end justify-center gap-4'>
          <div className='ft-tray rounded-lg flex flex-row items-end gap-2 px-3 pt-2 pb-1.5'>
            <DeckKey
              label='Rec'
              led={<Led color='red' state={recordLed} />}
            >
              <AnalogButton
                size='sm'
                latched={armed}
                aria-label='Arm recording'
                title="Arm recording (recording isn't connected yet)"
                onPress={() => setArmed(!armed)}
              >
                <RecordIcon className='ft-icon-rec' />
              </AnalogButton>
            </DeckKey>
            <DeckKey label='Rew'>
              <AnalogButton
                size='sm'
                latched={wind?.direction === -1}
                aria-label='Rewind'
                title='Rewind: tap to go back a bar, hold to wind'
                {...windKeyProps(-1)}
              >
                <RewindIcon />
              </AnalogButton>
            </DeckKey>
            <DeckKey label='F Fwd'>
              <AnalogButton
                size='sm'
                latched={wind?.direction === 1}
                aria-label='Fast forward'
                title='Fast forward: tap to go forward a bar, hold to wind'
                {...windKeyProps(1)}
              >
                <FastForwardIcon />
              </AnalogButton>
            </DeckKey>
            <DeckKey label='Play'>
              <AnalogButton
                size='sm'
                latched={playing}
                aria-label='Play'
                title='Play'
                onPress={() => dispatch(playPlaylist())}
              >
                <PlayIcon />
              </AnalogButton>
            </DeckKey>
            <DeckKey label='Stop'>
              <AnalogButton
                size='sm'
                aria-label='Stop'
                title='Stop, and go back to the start'
                onPress={() => {
                  assume(0)
                  dispatch(stopPlaylist()).then(refresh)
                }}
              >
                <StopIcon />
              </AnalogButton>
            </DeckKey>
            <DeckKey label='Pause'>
              <AnalogButton
                size='sm'
                latched={paused}
                aria-label='Pause'
                title={paused ? 'Carry on playing' : 'Pause'}
                onPress={() => dispatch(playing ? pausePlaylist() : playPlaylist())}
              >
                <PauseIcon />
              </AnalogButton>
            </DeckKey>
          </div>

          {/* what the tab shows */}
          <div className='flex flex-row items-end gap-2'>
            <LatchKey
              label={cassetteVisible ? 'Hide the cassette' : 'Show the cassette'}
              on={cassetteVisible}
              onToggle={() => dispatch(setCassetteVisible(!cassetteVisible))}
            >
              <TapeIcon />
            </LatchKey>
            {/* only means anything with the cassette showing */}
            {cassetteVisible && (
              <LatchKey
                label={tapePathVisible ? 'Hide the tape over the heads' : 'Show the tape over the heads'}
                on={tapePathVisible}
                onToggle={() => dispatch(setTapePathVisible(!tapePathVisible))}
              >
                <HeadIcon />
              </LatchKey>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

export default TransportDeck
