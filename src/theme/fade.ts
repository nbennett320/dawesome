// a theme change cross-fades from the old look to the new one (a view
// transition's default animation; its length is in global.scss)

type WithViewTransitions = Document & { startViewTransition?: (update: () => void) => unknown }

// runs `update` (which changes the theme) behind the fade, or straight away
// where there are no view transitions or less motion is asked for
export const fade = (update: () => void) => {
  const doc = document as WithViewTransitions
  const still = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches
  if(!doc.startViewTransition || still) {
    update()
    return
  }
  doc.startViewTransition(update)
}
