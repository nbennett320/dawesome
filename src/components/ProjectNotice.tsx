import React from 'react'
import { useAppDispatch, useAppSelector } from '../hooks/redux'
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

  return (
    <div
      role={missing || info ? 'status' : 'alert'}
      className={`flex flex-row items-start gap-3 px-4 py-2 text-xs border-b ${
        // eslint-disable-next-line no-nested-ternary
        info
          ? 'bg-slate-50 border-slate-200 text-slate-700'
          : missing
            ? 'bg-amber-50 border-amber-200 text-amber-900'
            : 'bg-red-50 border-red-200 text-red-900'
      }`}
    >
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
    </div>
  )
}

export default ProjectNotice
