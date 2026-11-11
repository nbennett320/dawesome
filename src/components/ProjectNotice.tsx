import React from 'react'
import { useAppDispatch, useAppSelector } from '../hooks/redux'
import { Notice } from './common'
import { selectProjectNotice, setProjectNotice } from '../state/slices/projectSlice'

// how many missing samples to list before summarising the rest
const LIST_AT_MOST = 5

// a dismissable banner for samples a project couldn't find, or a project
// command that failed
const ProjectNotice = () => {
  const notice = useAppSelector(selectProjectNotice)
  const dispatch = useAppDispatch()

  if(!notice) return null

  const missing = notice.kind === 'missing'
  const info = notice.kind === 'info'
  const listed = missing ? notice.samples.slice(0, LIST_AT_MOST) : []
  const more = missing ? notice.samples.length - listed.length : 0

  const shape = 'flex flex-row items-start gap-3 px-4 py-2 text-xs border-b'
  const body = (
    <>
      <div className='flex-1 min-w-0'>
        {missing ? (
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
                  <span className='text-amber-700'> ({sample.reason})</span>
                </li>
              ))}
            </ul>
            {more > 0 && <p className='mt-1'>…and {more} more.</p>}
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
    return <div role='status' className={`${shape} bg-slate-50 border-slate-200 text-slate-700`}>{body}</div>
  }

  return (
    <Notice tone={missing ? 'led' : 'danger'} role={missing ? 'status' : 'alert'} className={shape}>
      {body}
    </Notice>
  )
}

export default ProjectNotice
