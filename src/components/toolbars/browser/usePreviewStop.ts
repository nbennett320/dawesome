import React from 'react'
import { invoke } from '@tauri-apps/api/core'

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// a preview shorter than this is over too soon to need stopping, so it gets
// no stop button
export const STOPPABLE_SECS = 2

// previews a browser's sample or take, and keeps track of which row (by its
// key) is playing, so that row can show a stop button until it ends
export const usePreviewStop = () => {
  // the row being heard, while it plays
  const [playing, setPlaying] = React.useState<string | null>(null)
  // counts previews, so an older one's end can't hide a newer one's button,
  // and the timer that hides it when the preview ends
  const heard = React.useRef<{ count: number, timer?: ReturnType<typeof setTimeout> }>({ count: 0 })

  React.useEffect(() => () => clearTimeout(heard.current.timer), [])

  const preview = (key: string, path: string, label: string) => {
    // (the row heard before loses its button straight away)
    setPlaying(null)
    clearTimeout(heard.current.timer)
    const count = ++heard.current.count
    const current = () => heard.current.count === count
    // it answers with the file's length once it's playing (the first sound
    // opens the output, which takes a moment), so the button shows then
    invoke<number>('preview_sample', { path })
      .then(seconds => {
        if(!current() || seconds <= STOPPABLE_SECS) return
        setPlaying(key)
        heard.current.timer = setTimeout(() => setPlaying(null), seconds * 1000)
      })
      .catch(logError(`preview ${label}`))
  }

  const stop = () => {
    heard.current.count += 1
    clearTimeout(heard.current.timer)
    setPlaying(null)
    invoke('stop_previews').catch(logError('stop the preview'))
  }

  // the row playing is known by another key now (it was renamed), and its
  // button goes with it
  const rekey = (from: string, to: string) => setPlaying(key => (key === from ? to : key))

  return { playing, preview, stop, rekey }
}
