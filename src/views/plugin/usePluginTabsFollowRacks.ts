import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectRacks } from '../../state/slices/effectsSlice'
import { selectSynthRows } from '../../state/slices/sequencerSlice'
import { keepPluginTabs } from '../../state/slices/windowSlice'
import { tabSlotIds } from '../../types/effects'

// a plugin's tab closes when its slot goes from the racks, or its synth row
// from the sequencer (removed, undone away, another project), or it has
// nothing left to show
export const usePluginTabsFollowRacks = () => {
  const dispatch = useAppDispatch()
  const racks = useAppSelector(selectRacks)
  const synthRows = useAppSelector(selectSynthRows)

  React.useEffect(() => {
    dispatch(keepPluginTabs(tabSlotIds(racks, synthRows.map(row => row.instrument))))
  }, [dispatch, racks, synthRows])
}
