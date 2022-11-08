import React from 'react'
import { invoke } from '@tauri-apps/api'
import { ReactP5Wrapper } from 'react-p5-wrapper'
import sketch from './sketch'

const PlaylistCanvas = () => {
  const ref = React.useRef<HTMLDivElement>(null)
  const [height, setHeight] = React.useState<number>()
  const [width, setWidth] = React.useState<number>()
  const [limit, setLimit] = React.useState<number>(16)
  const [ratio, setRatio] = React.useState<number>(1)
  const range = [...Array(limit).keys()].map(e => e + 1)

  React.useEffect(() => {
    if(ref.current) {
      const { height: h, width: w } = ref.current.getBoundingClientRect()
      setHeight(h)
      setWidth(w)
    }
  }, [ref.current])

  React.useEffect(() => {
    const getPlaylistTimeline = async () => {
      const [maxPlaylistBeats, maxBeatsDisplayed, displayRatio] = 
        await invoke<[number, number, number]>('get_playlist_timeline', {})
      setLimit(maxPlaylistBeats)
      setRatio(displayRatio)
    }

    getPlaylistTimeline()
  }, [])

  return (
    <div 
      ref={ref}
      className='w-full h-full'
    >
      {ref.current && <ReactP5Wrapper 
        sketch={sketch}
        height={height}
        width={width}
        maxPlaylistBeats={limit}
      />}
    </div>
  )
}

export default PlaylistCanvas
