// short notes for the info bar along the bottom, from anywhere: where the
// keys just moved the playhead, say. the bar shows the latest one for a
// moment (see InfoBar)

type Listener = (text: string) => void

const listeners = new Set<Listener>()

export const postInfoNote = (text: string): void => {
  listeners.forEach(listener => listener(text))
}

// listen for notes. returns a function that stops listening
export const onInfoNote = (listener: Listener): (() => void) => {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}
