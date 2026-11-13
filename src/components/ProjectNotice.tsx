import React from 'react'
import { useAppDispatch, useAppSelector } from '../hooks/redux'
import { Notice } from './common'
import { trackLabel } from '../types/mixer'
import { selectProjectNotice, setProjectNotice } from '../state/slices/projectSlice'

// how many missing samples to list before summarising the rest
const LIST_AT_MOST = 5

// a dismissable banner for samples and plugins a project couldn't find, or a
// project command that failed
const ProjectNotice = () => {
  const notice = useAppSelector(selectProjectNotice)
  const dispatch = useAppDispatch()

  if(!notice) return null

  const missing = notice.kind === 'missing'
  const info = notice.kind === 'info'
  const listed = missing ? notice.samples.slice(0, LIST_AT_MOST) : []
  const more = missing ? notice.samples.length - listed.length : 0
  // (older notices may not have the effects, synths or what was dropped)
  const effects = missing ? notice.effects ?? [] : []
  const listedEffects = effects.slice(0, LIST_AT_MOST)
  const moreEffects = effects.length - listedEffects.length
  const synths = missing ? notice.synths ?? [] : []
  const dropped = missing ? notice.dropped ?? [] : []

  const shape = 'flex flex-row items-start gap-3 px-4 py-2 text-xs border-b'
  const body = (
    <>
      <div className='flex-1 min-w-0'>
        {missing ? (
          <>
            {notice.samples.length > 0 && (
              <>
                <p>
                  {notice.samples.length === 1
                    ? '1 clip was left out because its sample couldn’t be loaded:'
                    : `${notice.samples.length} clips were left out because their samples couldn’t be loaded:`}
                </p>
                <ul className='mt-1 list-disc pl-5'>
                  {listed.map((sample, i) => (
                    // eslint-disable-next-line react/no-array-index-key
                    <li key={i} className='truncate' title={sample.path}>
                      <span className='font-mono'>{sample.path}</span>
                      <span className='text-led-ink'> ({sample.reason})</span>
                    </li>
                  ))}
                </ul>
                {more > 0 && <p className='mt-1'>…and {more} more.</p>}
              </>
            )}
            {effects.length > 0 && (
              <>
                <p className={notice.samples.length > 0 ? 'mt-2' : undefined}>
                  These plugins aren’t installed, so their effects are passing sound straight through (their settings are kept):
                </p>
                <ul className='mt-1 list-disc pl-5'>
                  {listedEffects.map((effect, i) => (
                    // eslint-disable-next-line react/no-array-index-key
                    <li key={i} className='truncate' title={`${effect.path} (${effect.reason})`}>
                      {`${effect.name}${effect.vendor ? ` (${effect.vendor})` : ''} on ${effect.track === null ? 'the master' : trackLabel(effect.track)}`}
                    </li>
                  ))}
                </ul>
                {moreEffects > 0 && <p className='mt-1'>…and {moreEffects} more.</p>}
              </>
            )}
            {synths.length > 0 && (
              <>
                <p className={notice.samples.length > 0 || effects.length > 0 ? 'mt-2' : undefined}>
                  {synths.length === 1
                    ? 'This instrument isn’t installed, so its notes are silent (its settings are kept):'
                    : 'These instruments aren’t installed, so their notes are silent (their settings are kept):'}
                </p>
                <ul className='mt-1 list-disc pl-5'>
                  {synths.map(synth => (
                    <li key={synth.slot} className='truncate'>
                      {`${synth.name}${synth.vendor ? ` (${synth.vendor})` : ''} in the Pattern Sequencer`}
                    </li>
                  ))}
                </ul>
              </>
            )}
            {dropped.map((sentence, i) => (
              // eslint-disable-next-line react/no-array-index-key
              <p key={i} className={i === 0 && notice.samples.length + effects.length + synths.length > 0 ? 'mt-2' : undefined}>
                {sentence}
              </p>
            ))}
          </>
        ) : (
          <p>{notice.message}</p>
        )}
      </div>

      <button
        type='button'
        className='shrink-0 px-1 leading-none text-base opacity-60 hover:opacity-100'
        aria-label='Dismiss'
        onClick={() => dispatch(setProjectNotice(null))}
      >
        ×
      </button>
    </>
  )

  // (info isn't a Notice tone: it's neither a warning nor a failure)
  if(info) {
    return <div role='status' className={`${shape} bg-surface-alt border-line text-ink`}>{body}</div>
  }

  return (
    <Notice tone={missing ? 'led' : 'danger'} role={missing ? 'status' : 'alert'} className={shape}>
      {body}
    </Notice>
  )
}

export default ProjectNotice
