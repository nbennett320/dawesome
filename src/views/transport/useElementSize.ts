import React from 'react'

export interface Size {
  width: number
  height: number
}

// an element's content box, kept up to date as it resizes. null until it's
// first measured
export const useElementSize = (ref: React.RefObject<HTMLElement>): Size | null => {
  const [size, setSize] = React.useState<Size | null>(null)

  React.useEffect(() => {
    const el = ref.current
    if(!el || typeof ResizeObserver === 'undefined') return undefined

    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect
      setSize(prev => (prev && prev.width === width && prev.height === height ? prev : { width, height }))
    })
    observer.observe(el)
    return () => observer.disconnect()
  }, [ref])

  return size
}
