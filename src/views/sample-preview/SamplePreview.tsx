import React from 'react'
import { selectInspectedClip } from '../../state/slices/playlistSlice'
import { useAppSelector } from '../../hooks/redux'

// inspect a clip from the playlist and adjust how it plays
// (the clip's details for now, the settings come next)
const SamplePreview = () => {
  const clip = useAppSelector(selectInspectedClip)

  if(!clip) {
    return (
      <div className='h-full w-full flex items-center justify-center text-sm text-gray-500'>
        Double-click a clip in the playlist to inspect it.
      </div>
    )
  }

  const name = clip.path.split(/[\\/]/).pop() ?? clip.path

  return (
    <div className='h-full w-full p-4 text-sm'>
      <h2 className='text-base font-medium truncate' title={clip.path}>{name}</h2>

      <dl className='mt-2 grid gap-x-4 gap-y-1 text-xs' style={{ gridTemplateColumns: 'max-content 1fr' }}>
        <dt className='text-gray-500'>Track</dt>
        {/* tracks are numbered from 0 in the playlist */}
        <dd>{clip.trackNumber + 1}</dd>

        <dt className='text-gray-500'>Starts at beat</dt>
        <dd>{Number(clip.startBeat.toFixed(2))}</dd>

        <dt className='text-gray-500'>File</dt>
        <dd className='truncate' title={clip.path}>{clip.path}</dd>
      </dl>
    </div>
  )
}

export default SamplePreview
