import React from 'react'
import { invoke } from '@tauri-apps/api/core'
import { useAppDispatch, useAppSelector } from '../hooks/redux'
import { fileName } from '../util/helpers'
import { Notice } from './common'
import { trackLabel } from '../types/mixer'
import { selectProjectNotice, setProjectNotice } from '../state/slices/projectSlice'

// how many missing samples to list before summarising the rest
const LIST_AT_MOST = 5

const percent = (done: number, total: number) => (total > 0 ? Math.floor((done / total) * 100) : 0)

// a shareable project being packed, then where it was saved, with the
// samples it couldn't take and the plugins whoever opens it needs
const ShareNotice = () => {
  const notice = useAppSelector(selectProjectNotice)
  const dispatch = useAppDispatch()
  const [problem, setProblem] = React.useState<string | null>(null)
  if(notice?.kind !== 'sharing' && notice?.kind !== 'shared') return null

  const shape = 'flex flex-row items-start gap-3 px-4 py-2 text-xs border-b bg-surface-alt border-line text-ink'
  if(notice.kind === 'sharing') {
    return (
      <div role='status' className={shape}>
        <p className='flex-1 min-w-0'>
          Packing {notice.file}… {percent(notice.done, notice.total)}%
        </p>
      </div>
    )
  }

  const { path, missing, plugins } = notice.share
  const reveal = () => {
    setProblem(null)
    invoke('reveal_file', { path }).catch(e => setProblem(String(e)))
  }
  return (
    <div role='status' className={shape}>
      <div className='flex-1 min-w-0'>
        <p>
          Saved <span className='font-medium'>{fileName(path)}</span>, with every sample the project uses. Send it to anyone with Dawesome.
          {' '}
          <button type='button' className='underline' onClick={reveal}>Show in Finder</button>
        </p>
        {missing.length > 0 && (
          <>
            <p className='mt-2 text-led-ink'>
              {missing.length === 1 ? 'This sample couldn’t be found, so it isn’t in it:' : 'These samples couldn’t be found, so they aren’t in it:'}
            </p>
            <ul className='mt-1 list-disc pl-5'>
              {missing.slice(0, LIST_AT_MOST).map(sample => (
                <li key={sample} className='truncate font-mono' title={sample}>{sample}</li>
              ))}
            </ul>
            {missing.length > LIST_AT_MOST && <p className='mt-1'>…and {missing.length - LIST_AT_MOST} more.</p>}
          </>
        )}
        {plugins.length > 0 && (
          <>
            <p className='mt-2'>Plugins aren’t included. Whoever opens it needs these installed too:</p>
            <ul className='mt-1 list-disc pl-5'>
              {plugins.map(plugin => <li key={plugin} className='truncate'>{plugin}</li>)}
            </ul>
          </>
        )}
        {problem && <p className='mt-2 text-danger-ink'>{problem}</p>}
      </div>
      <button
        type='button'
        className='shrink-0 px-1 leading-none text-base opacity-60 hover:opacity-100'
        aria-label='Dismiss'
        onClick={() => dispatch(setProjectNotice(null))}
      >
        ×
      </button>
    </div>
  )
}

// a dismissable banner for samples and plugins a project couldn't find, or a
// project command that failed
const ProjectNotice = () => {
  const notice = useAppSelector(selectProjectNotice)
  const dispatch = useAppDispatch()

  if(!notice) return null
  if(notice.kind === 'sharing' || notice.kind === 'shared') return <ShareNotice />

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
