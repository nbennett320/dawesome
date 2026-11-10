import React from 'react'
import { selectPlaylistTempo, validTempo } from '../../state/slices/playlistSlice'
import { changeTempo } from '../../state/slices/sequencerSlice'
import { selectRecording } from '../../state/slices/recordingSlice'
import { useAppSelector, useAppDispatch } from '../../hooks/redux'

// how long after the last keystroke a typed tempo is taken up. (not on each
// keystroke: on the way to 100, "1" and "10" are tempos too, and playback
// would lurch through them)
const TAKE_UP_AFTER_MS = 500

const TempoInput = () => {
  const tempo = useAppSelector(selectPlaylistTempo)
  // the tempo can't change under a take
  const recording = useAppSelector(selectRecording)
  const dispatch = useAppDispatch()
  // what's been typed, while the field has the keyboard. it's taken up a
  // moment after typing stops, on Enter, or when the field is left
  const [draft, setDraft] = React.useState<string | null>(null)
  const timer = React.useRef<ReturnType<typeof setTimeout> | null>(null)

  const cancel = () => {
    if(timer.current) clearTimeout(timer.current)
    timer.current = null
  }

  const takeUp = (value: string) => {
    cancel()
    const typed = parseFloat(value)
    if(!validTempo(typed) || typed === tempo) return

    dispatch(changeTempo(typed)).then(changed => {
      // refused: back to the tempo it is
      if(!changed) setDraft(null)
    })
  }

  // the tempo changed some other way (a project was opened), or a recording
  // started: what was being typed no longer applies
  React.useEffect(() => {
    setDraft(typed => (typed !== null && (recording || parseFloat(typed) !== tempo) ? null : typed))
    if(recording) cancel()
  }, [tempo, recording])

  React.useEffect(() => cancel, [])

  return (
    <div className='flex flex-row items-center gap-1.5'>
      <span className='ft-label'>Tempo</span>
      <input
        type='number'
        className='ft-readout text-sm py-0.5 w-14'
        aria-label='Playlist tempo input'
        disabled={recording}
        title={recording ? 'The tempo can\u2019t be changed while recording' : undefined}
        value={draft ?? tempo}
        onFocus={() => setDraft(String(tempo))}
        onBlur={e => {
          takeUp(e.currentTarget.value)
          setDraft(null)
        }}
        onKeyDown={e => {
          if(e.key === 'Enter') takeUp(e.currentTarget.value)
        }}
        onInput={e => {
          const { value } = e.currentTarget
          setDraft(value)
          cancel()
          timer.current = setTimeout(() => takeUp(value), TAKE_UP_AFTER_MS)
        }}
      />
    </div>
  )
}

export default TempoInput
