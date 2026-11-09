import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  pausePlaylist,
  playPlaylist,
  seekPlaylist,
  selectPlaylistPlaying,
  stopPlaylist,
} from '../../state/slices/playlistSlice'
import { selectCassetteVisible, setCassetteVisible } from '../../state/slices/windowSlice'
import { projectName, selectProjectPath } from '../../state/slices/projectSlice'
import {
  AnalogButton,
  Cassette,
  FastForwardIcon,
  Led,
  LedState,
  PauseIcon,
  PlayIcon,
  RecordIcon,
  RewindIcon,
  StopIcon,
  TapeCounter,
} from '../../components/fourtrack'
import LatchKey from '../../components/topbar/LatchKey'
import {
  barAndBeat,
  formatTime,
  positionAt,
  tapeLengthMs,
  tapTarget,
  Wind,
  windDone,
  windPosition,
  WIND_SPEED,
} from './tape'
import { useTicker, useTransport } from './useTransport'

// holding a wind key this long winds the tape; a shorter press moves a bar
const HOLD_MS = 250

// a key with its name printed under it, and room for an LED above
const DeckKey = ({ label, led, children }: { label: string, led?: React.ReactNode, children: React.ReactNode }) => (
  <div className='flex flex-col items-center gap-1'>
    {led ?? <span className='h-2' aria-hidden />}
    {children}
    <span className='ft-label text-[9px]'>{label}</span>
  </div>
)

const TapeIcon = () => (
  <svg viewBox='0 0 24 24' fill='none' stroke='currentColor' strokeWidth='1.75' aria-hidden>
    <rect x='3' y='6' width='18' height='12' rx='2' />
    <circle cx='9' cy='12' r='1.75' />
    <circle cx='15' cy='12' r='1.75' />
  </svg>
)

// a cassette and the keys of a tape deck: record, rewind, fast forward, play,
// stop and pause. the wind keys move a bar when tapped, and wind the tape
// while held
const TransportDeck = () => {
  const dispatch = useAppDispatch()
  const playing = useAppSelector(selectPlaylistPlaying)
  const cassetteVisible = useAppSelector(selectCassetteVisible)
  const label = projectName(useAppSelector(selectProjectPath)) ?? 'untitled'
  const { transport, refresh, assume } = useTransport()
  const [wind, setWind] = React.useState<Wind | null>(null)
  // recording isn't connected to the engine yet, so arming only lights the key
  const [armed, setArmed] = React.useState(false)

  useTicker(playing || wind !== null)

  let position = 0
  if(transport) {
    position = wind ? windPosition(wind, performance.now(), transport) : positionAt(transport, Date.now())
  }
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
      if((e.key === 'Enter' || e.key === ' ') && !e.repeat) {
        e.preventDefault()
        press(direction)
      }
    },
    onKeyUp: (e: React.KeyboardEvent) => {
      if(e.key === 'Enter' || e.key === ' ') release()
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

  const { bar, beat } = transport ? barAndBeat(position, transport) : { bar: 1, beat: 1 }

  return (
    <div className='flex flex-col items-center gap-5'>
      {cassetteVisible && (
        <div className='w-full max-w-md'>
          <Cassette progress={progress} speed={speed} label={label} />
        </div>
      )}

      <div className='flex flex-row flex-wrap items-end justify-center gap-4'>
        <div className='ft-lcd grid grid-cols-[auto_auto_auto] gap-x-3 gap-y-1 items-center'>
          <span className='ft-label text-[9px]'>Bar</span>
          <span className='ft-label text-[9px]'>Beat</span>
          <span className='ft-label text-[9px]'>Time</span>
          <TapeCounter value={bar} digits={3} label='Bar' />
          <TapeCounter value={beat} digits={1} label='Beat' />
          <span className='text-sm tabular-nums min-w-[4rem] text-right'>{formatTime(position)}</span>
        </div>

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

        <LatchKey
          label={cassetteVisible ? 'Hide the cassette' : 'Show the cassette'}
          on={cassetteVisible}
          onToggle={() => dispatch(setCassetteVisible(!cassetteVisible))}
        >
          <TapeIcon />
        </LatchKey>
      </div>
    </div>
  )
}

export default TransportDeck
