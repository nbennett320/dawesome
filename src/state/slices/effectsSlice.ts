import { createSlice, Dispatch, PayloadAction } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import type { RootState } from 'state/store'
import {
  AddedEffect,
  ChannelId,
  EditorInfo,
  EffectSlot,
  findSlot,
  fxLit,
  isInstrument,
  PluginCatalog,
  Racks,
  Rect,
  slotsOf,
  toTrack,
} from '../../types/effects'
import { LoadedProject } from '../../types/project'
import { tabKey, View } from '../../types/ui'
import { paneWithTab } from '../paneTree'
import { reduceLoadedProject, reduceProjectView } from './playlistSlice'
import { setProjectNotice } from './projectSlice'
import { closeTab, openPluginTab, openTab } from './windowSlice'

// effect plugins: the ones the engine found, every channel's chain of them,
// and what the Effects Rack tab shows. the engine owns the chains, and every
// command that changes one hands back all of them, which replace these

export interface EffectsState {
  // what the last scan found, or null before it's been asked
  catalog: PluginCatalog | null
  racks: Racks
  // the channel the Effects Rack tab shows
  rackChannel: ChannelId
  // a slot to point out in the stack for a moment, e.g. one just dropped in
  highlightSlotId: number | null
}

const initialState: EffectsState = {
  catalog: null,
  racks: [],
  rackChannel: 0,
  highlightSlotId: null,
}

// after the racks change: a highlight goes with its slot (and a plugin's
// tab with its slot, see usePluginTabsFollowRacks)
const tidy = (state: EffectsState) => {
  const { highlightSlotId } = state

  if(highlightSlotId !== null && !findSlot(state.racks, highlightSlotId)) {
    state.highlightSlotId = null
  }
}

export const effectsSlice = createSlice({
  name: 'effectsSlice',
  initialState,
  reducers: {
    setCatalog: (state, action: PayloadAction<PluginCatalog>) => {
      state.catalog = action.payload
    },
    setRacks: (state, action: PayloadAction<Racks>) => {
      state.racks = action.payload
      tidy(state)
    },
    // show a channel in the rack tab
    showRackChannel: (state, action: PayloadAction<ChannelId>) => {
      state.rackChannel = action.payload
    },
    highlightSlot: (state, action: PayloadAction<number | null>) => {
      state.highlightSlotId = action.payload
    },
  },
  extraReducers: builder => {
    // a project brings its own effects (an older engine sends none)
    builder.addCase(reduceLoadedProject, (state, action: PayloadAction<LoadedProject>) => {
      state.racks = action.payload.effects ?? []
      tidy(state)
    })
    // and an undo or redo may have changed them
    builder.addCase(reduceProjectView, (state, action) => {
      state.racks = action.payload.project.effects ?? []
      tidy(state)
    })
  },
})

export const { setCatalog, setRacks, showRackChannel, highlightSlot } = effectsSlice.actions

export const selectCatalog = (state: RootState) => state.effects.catalog
export const selectRacks = (state: RootState) => state.effects.racks
export const selectRackChannel = (state: RootState) => state.effects.rackChannel
export const selectHighlightSlotId = (state: RootState) => state.effects.highlightSlotId

// a channel's chain, in the order the sound goes through it
export const selectChannelSlots = (channel: ChannelId) => (state: RootState): EffectSlot[] =>
  slotsOf(state.effects.racks, channel)
export const selectChannelCount = (channel: ChannelId) => (state: RootState): number =>
  slotsOf(state.effects.racks, channel).length
// whether a channel has an instrument (loaded or not)
export const selectChannelHasInstrument = (channel: ChannelId) => (state: RootState): boolean =>
  slotsOf(state.effects.racks, channel).some(isInstrument)
// whether any of a channel's effects is on (not bypassed, and loaded)
export const selectChannelActive = (channel: ChannelId) => (state: RootState): boolean =>
  fxLit(slotsOf(state.effects.racks, channel))

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// a command that failed is shown above the workspace, like the project's
const fail = (dispatch: Dispatch, what: string) => (e: unknown) => {
  dispatch(setProjectNotice({ kind: 'error', message: `${what}: ${message(e)}` }))
}

const logError = (what: string) => (e: unknown) =>
  // eslint-disable-next-line no-console
  console.error(`could not ${what}:`, e)

// the plugins found so far (and how a scan is going)
export const fetchPlugins = () => async (dispatch: Dispatch): Promise<PluginCatalog> => {
  const catalog = await invoke<PluginCatalog>('get_plugins', {})
  dispatch(setCatalog(catalog))
  return catalog
}

// look for plugins again, in the background: `everything` rescans those
// already known (and those that failed) too. poll fetchPlugins while it goes
export const scanPlugins = (everything: boolean) => async (dispatch: Dispatch): Promise<void> => {
  await invoke<void>('scan_plugins', { everything })
  await fetchPlugins()(dispatch)
}

export const fetchEffects = () => async (dispatch: Dispatch): Promise<void> => {
  dispatch(setRacks(await invoke<Racks>('get_effects', {})))
}

// put an effect on a channel, at `index` in its chain or (null) at the end.
// resolves to the new slot's id, or null when the plugin couldn't be loaded
// (which is shown, and changes nothing)
export const addEffect = (channel: ChannelId, pluginId: string, index: number | null) =>
  async (dispatch: Dispatch): Promise<number | null> => {
    try {
      const added = await invoke<AddedEffect>('add_effect', { track: toTrack(channel), pluginId, index })
      dispatch(setRacks(added.racks))
      return added.slotId
    } catch(e) {
      fail(dispatch, 'Couldn’t add the effect')(e)
      return null
    }
  }

export const removeEffect = (slotId: number) => async (dispatch: Dispatch): Promise<void> => {
  try {
    dispatch(setRacks(await invoke<Racks>('remove_effect', { slotId })))
  } catch(e) {
    fail(dispatch, 'Couldn’t remove the effect')(e)
  }
}

// move an effect within its chain or to another channel's, settings kept.
// `index` is where it is in the target chain after the move
export const moveEffect = (slotId: number, channel: ChannelId, index: number) =>
  async (dispatch: Dispatch): Promise<void> => {
    try {
      dispatch(setRacks(await invoke<Racks>('move_effect', { slotId, track: toTrack(channel), index })))
    } catch(e) {
      fail(dispatch, 'Couldn’t move the effect')(e)
    }
  }

export const setEffectBypassed = (slotId: number, bypassed: boolean) =>
  async (dispatch: Dispatch): Promise<void> => {
    try {
      dispatch(setRacks(await invoke<Racks>('set_effect_bypassed', { slotId, bypassed })))
    } catch(e) {
      fail(dispatch, `Couldn’t ${bypassed ? 'bypass' : 'switch on'} the effect`)(e)
    }
  }

// the effect's own interface in a floating window. it's in one place at a
// time, so its tab closes
export const openEffectWindow = (slotId: number) =>
  async (dispatch: Dispatch): Promise<void> => {
    dispatch(closeTab(tabKey(View.Plugin, slotId)))

    try {
      await invoke<void>('open_effect_window', { slotId })
    } catch(e) {
      fail(dispatch, 'Couldn’t open the effect’s window')(e)
    }
  }

export const closeEffectWindow = (slotId: number) => async (): Promise<void> => {
  await invoke<void>('close_effect_window', { slotId }).catch(logError('close the effect’s window'))
}

// the room a resizable interface may fill
export interface Size {
  width: number
  height: number
}

// an effect's own interface, inside the main window (in its tab): `clip` is
// the part of the page it may cover, `x`, `y` where its top-left corner
// goes, and `fill` the room it has. several can be docked at once. resolves
// to the size it took, rejects when it can't be (shown by the caller)
export const dockEffect = (slotId: number, clip: Rect, x: number, y: number, fill: Size | null) =>
  (): Promise<EditorInfo> => invoke<EditorInfo>('dock_effect', { slotId, clip, x, y, fill })

// the page moved under a docked interface
export const moveDockedEffect = (slotId: number, clip: Rect, x: number, y: number) =>
  async (): Promise<void> => {
    await invoke<void>('move_docked_effect', { slotId, clip, x, y }).catch(logError('move the docked effect'))
  }

// a resizable docked interface's room changed. resolves to the size it took
export const resizeDockedEffect = (slotId: number, { width, height }: Size) => (): Promise<EditorInfo> =>
  invoke<EditorInfo>('resize_docked_effect', { slotId, width, height })

// hide a docked interface
export const undockEffect = (slotId: number) => async (): Promise<void> => {
  await invoke<void>('undock_effect', { slotId }).catch(logError('undock the effect'))
}

// open the Effects Rack tab (or select it where it's open) on a channel
export const showRack = (channel: ChannelId) => (dispatch: Dispatch) => {
  dispatch(showRackChannel(channel))
  dispatch(openTab(View.EffectsRack))
}

// open the rack on a slot's channel, with its row pointed out
export const showInRack = (slotId: number) => (dispatch: Dispatch, getState: () => RootState) => {
  const found = findSlot(getState().effects.racks, slotId)
  if(!found) return

  showRack(found.channel)(dispatch)
  dispatch(highlightSlot(slotId))
}

// open an effect's own tab in the pane the rack is in (or select it where
// it's open). docking it there closes its window
export const showPluginTab = (slotId: number) => (dispatch: Dispatch, getState: () => RootState) => {
  const rack = paneWithTab(getState().window.windowPane.root, tabKey(View.EffectsRack))
  dispatch(openPluginTab({ slotId, paneId: rack?.id }))
}

// open an effect's own interface where the preferences say plugins open: a
// window of its own, or a tab
export const openPlugin = (slotId: number) =>
  async (dispatch: Dispatch, getState: () => RootState): Promise<void> => {
    if(getState().appearance.preferences.pluginView === 'tab') {
      showPluginTab(slotId)(dispatch, getState)
      return
    }

    await openEffectWindow(slotId)(dispatch)
  }

// an effect from the browser dropped on a mixer channel: it goes at the end
// of the chain, and the rack opens on that channel with it pointed out
export const dropEffectOnChannel = (channel: ChannelId, pluginId: string) =>
  async (dispatch: Dispatch): Promise<number | null> => {
    const slotId = await addEffect(channel, pluginId, null)(dispatch)
    if(slotId === null) return null

    showRack(channel)(dispatch)
    dispatch(highlightSlot(slotId))
    return slotId
  }

export default effectsSlice.reducer
