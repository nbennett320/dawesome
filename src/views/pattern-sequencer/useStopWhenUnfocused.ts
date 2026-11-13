import React from 'react'
import { useStore } from 'react-redux'
import { useAppDispatch } from '../../hooks/redux'
import type { RootState } from '../../state/store'
import { isPatternView, stopPattern } from '../../state/slices/sequencerSlice'
import { selectFocusedView } from '../../state/slices/windowSlice'

// one stop at a time, however many of the tabs are open
let stopping = false

// a pattern playing on its own stops once neither the Pattern Sequencer
// nor the Piano Roll is the focused pane's tab: another tab selected,
// another pane clicked, or the tab closed. (going from one to the other
// keeps it playing)
export const useStopWhenUnfocused = () => {
  const store = useStore<RootState>()
  const dispatch = useAppDispatch()

  React.useEffect(() => {
    const check = () => {
      const state = store.getState()
      if(stopping || !state.sequencer.run || isPatternView(selectFocusedView(state))) return
      stopping = true
      dispatch(stopPattern()).finally(() => { stopping = false })
    }

    const unsubscribe = store.subscribe(check)
    return () => {
      unsubscribe()
      check()
    }
  }, [store, dispatch])
}
