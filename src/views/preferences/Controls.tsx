import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { changePreferences, selectAppearance } from '../../state/slices/appearanceSlice'
import { Choice } from '../../components/common'
import { AnalogButton, Led } from '../../components/fourtrack'
import type { Preferences as Prefs } from '../../theme'

const PLUGIN_VIEWS = [
  { value: 'window', label: 'In a window of their own' },
  { value: 'tab', label: 'In a tab' },
]

const LOOP_RECORDING = [
  { value: 'layer', label: 'Keep recording, layering each pass' },
  { value: 'stop', label: 'Stop recording and playback' },
]

// how tracks on tape are worked out (T23 in context/tape-project)
const TAPE_MODES = [
  { value: 'print', label: 'Print in the background' },
  { value: 'live', label: 'Always live' },
]

const TAPE_MODE_HINT = 'Print works out the tape\'s saturation once, in the background, whenever a track or the tape\'s settings change: light on the CPU. '
  + 'Until a track is printed it\'s heard direct, as a 4-track\'s input is before it\'s on tape. '
  + 'Always live works it out all the time, so changes are heard at once, but costs more CPU for each track on tape'

const TAPE_METHODS = [
  { value: 'modeled', label: 'Computer modeled' },
  { value: 'neural', label: 'Neural' },
]

const TAPE_METHOD_HINT = 'Computer modeled: a hand-built model of a deck\'s saturation and tone, light on the CPU. '
  + 'Neural: a network trained on recordings of a deck, heavier on the CPU (for now trained on a stand-in deck, and only at 48 kHz)'

const TAPE_TAPS = [
  { value: 'before', label: 'Before its effects' },
  { value: 'after', label: 'After its effects' },
]

const TAPE_TAP_HINT = 'Before its effects, as on a 4-track: the track goes onto tape, and its effects work on what comes off it. '
  + 'After its effects: the track\'s effects are recorded onto tape too'

// whether Stop and Play run the tape down and up (T30), most first
const TAPE_INERTIA = [
  { value: 'both', label: 'On stop and start' },
  { value: 'stop', label: 'On stop only' },
  { value: 'start', label: 'On start only' },
  { value: 'off', label: 'Off' },
]

const TAPE_INERTIA_HINT = 'With the tape on, the transport\'s Stop and Play can go as a deck\'s motor does: '
  + 'Stop lets the tape coast down to a standstill, its pitch diving, and Play spins it up from one. '
  + 'Off, they\'re at once. Not while recording'

// which drags are heard scrubbing (T32)
const SCRUBBING = [
  { value: 'on', label: 'On' },
  { value: 'transport', label: 'Timeline and transport' },
  { value: 'off', label: 'Off' },
]

const SCRUBBING_HINT = 'Scrubbing plays the song as the playhead\'s dragged, at the drag\'s speed, as a tape rocked over the heads. '
  + 'On: dragging the playhead in the Timeline and the playlist, the Transport tab\'s reels, and holding Rewind or Fast forward. '
  + 'Timeline and transport: all but the playlist\'s playhead (dragging that only moves it). Off: none, and no copy of the song is kept for it (saves CPU and memory)'

// how the app is worked: where plugins open, the command bar, what a
// recording does at the loop's end, and how tracks on tape are worked out.
// changes apply as they're made
const Controls = () => {
  const dispatch = useAppDispatch()
  const { preferences } = useAppSelector(selectAppearance)
  const { commandBar } = preferences

  return (
    <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
      <div className='flex flex-col gap-4 max-w-xl'>
        <section className='flex flex-col gap-3'>
          <Choice
            id='preferences-plugin-view'
            label='Plugins open'
            value={preferences.pluginView}
            options={PLUGIN_VIEWS}
            onChange={pluginView => dispatch(changePreferences({ pluginView: pluginView as Prefs['pluginView'] }))}
          />
          <Choice
            id='preferences-loop-recording'
            label="At the loop's end, while recording"
            value={preferences.loopRecording}
            options={LOOP_RECORDING}
            onChange={loopRecording => dispatch(changePreferences({ loopRecording: loopRecording as Prefs['loopRecording'] }))}
          />
          <div className='flex flex-col gap-1'>
            <span className='ft-label'>Command bar</span>
            <div className='flex flex-row items-center gap-2'>
              <AnalogButton
                size='xs'
                latched={commandBar}
                aria-label='Command bar'
                data-hint='A vim-style command bar: type : to open it along the bottom. See the docs for the available commands'
                onPress={() => dispatch(changePreferences({ commandBar: !commandBar }))}
              />
              <Led color='blue' state={commandBar ? 'on' : 'off'} />
              <span>{commandBar ? 'On' : 'Off'}</span>
            </div>
          </div>
        </section>
        <section className='flex flex-col gap-3'>
          <span className='ft-label'>Tape</span>
          <div data-hint={TAPE_MODE_HINT}>
            <Choice
              id='preferences-tape-mode'
              label='Tracks on tape'
              value={preferences.tapeMode}
              options={TAPE_MODES}
              onChange={tapeMode => dispatch(changePreferences({ tapeMode: tapeMode as Prefs['tapeMode'] }))}
            />
          </div>
          <div data-hint={TAPE_METHOD_HINT}>
            <Choice
              id='preferences-tape-method'
              label='Tape emulation'
              value={preferences.tapeMethod}
              options={TAPE_METHODS}
              onChange={tapeMethod => dispatch(changePreferences({ tapeMethod: tapeMethod as Prefs['tapeMethod'] }))}
            />
          </div>
          <div data-hint={TAPE_TAP_HINT}>
            <Choice
              id='preferences-tape-tap'
              label='Where a track meets the tape'
              value={preferences.tapeTap}
              options={TAPE_TAPS}
              onChange={tapeTap => dispatch(changePreferences({ tapeTap: tapeTap as Prefs['tapeTap'] }))}
            />
          </div>
          <div data-hint={SCRUBBING_HINT}>
            <Choice
              id='preferences-scrub'
              label='Scrubbing playback'
              value={preferences.scrub}
              options={SCRUBBING}
              onChange={scrub => dispatch(changePreferences({ scrub: scrub as Prefs['scrub'] }))}
            />
          </div>
          <div data-hint={TAPE_INERTIA_HINT}>
            <Choice
              id='preferences-tape-inertia'
              label='Tape stop and start'
              value={preferences.tapeInertia}
              options={TAPE_INERTIA}
              onChange={tapeInertia => dispatch(changePreferences({ tapeInertia: tapeInertia as Prefs['tapeInertia'] }))}
            />
          </div>
        </section>
      </div>
    </div>
  )
}

export default Controls
