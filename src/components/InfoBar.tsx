import React from 'react'

// the strip along the bottom: describes whatever the mouse is over. anything
// can offer a description with a data-hint attribute, or falls back on its
// title (tooltip); the nearest one up from the hovered element wins
const InfoBar = () => {
  const [hint, setHint] = React.useState('')

  React.useEffect(() => {
    const onOver = (e: MouseEvent) => {
      const el = (e.target as Element | null)?.closest?.('[data-hint], [title]')
      setHint(el?.getAttribute('data-hint') ?? el?.getAttribute('title') ?? '')
    }
    // leaving the window clears it
    const onLeave = () => setHint('')

    document.addEventListener('mouseover', onOver)
    document.documentElement.addEventListener('mouseleave', onLeave)
    return () => {
      document.removeEventListener('mouseover', onOver)
      document.documentElement.removeEventListener('mouseleave', onLeave)
    }
  }, [])

  return (
    <div className="InfoBar flex items-center px-3 border-t border-line bg-panel text-xs text-ink-soft truncate">
      {hint}
    </div>
  )
}

export default InfoBar
