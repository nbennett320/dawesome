import React from 'react'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import { selectRacks } from '../../state/slices/effectsSlice'
import { keepPluginTabs } from '../../state/slices/windowSlice'
import { tabSlotIds } from '../../types/effects'

// a plugin's tab closes when its slot goes from the racks (removed, undone
// away, another project) or has nothing left to show
export const usePluginTabsFollowRacks = () => {
  const dispatch = useAppDispatch()
  const racks = useAppSelector(selectRacks)

  React.useEffect(() => {
    dispatch(keepPluginTabs(tabSlotIds(racks)))
  }, [dispatch, racks])
}
