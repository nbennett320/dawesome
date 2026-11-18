import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import { openTab } from '../state/slices/windowSlice'
import { ProjectCommand, confirmQuit, openProjectFile, openRecentProject, runProjectCommand } from '../state/slices/projectSlice'
import type { AppDispatch } from '../state/store'
import { View } from '../types/ui'

// the File menu's project items. resolves to a function that stops listening
const projectMenu = (dispatch: AppDispatch) =>
  listen<{ data: ProjectCommand }>('menu-project', ev => {
    dispatch(runProjectCommand(ev.payload.data))
  })

// File › Open Recent's projects. resolves to a function that stops listening
const recentProjectMenu = (dispatch: AppDispatch) =>
  listen<{ data: string }>('menu-open-recent', ev => {
    dispatch(openRecentProject(ev.payload.data))
  })

// projects (or shareable projects) double-clicked in Finder: one that
// opened the app is held by the engine until asked for, once listening (so
// none is missed), and later ones are sent as they come. only one project
// is open at a time, so of several, the first. resolves to a function that
// stops listening
const openedFiles = async (dispatch: AppDispatch) => {
  const stop = await listen<string>('open-file', ev => {
    dispatch(openProjectFile(ev.payload))
  })
  const held = await invoke<string[]>('opened_files').catch(() => [])
  if(held.length) dispatch(openProjectFile(held[0]))
  return stop
}

// quitting (or closing the window) with unsaved changes: the engine waits
// for the frontend to ask about them. resolves to a function that stops
// listening
const quitRequested = (dispatch: AppDispatch) =>
  listen('quit-requested', () => {
    dispatch(confirmQuit())
  })

// the Window menu's items, by the names the engine sends
export const menuViews: Record<string, View> = {
  playlist: View.Playlist,
  sample: View.SamplePreview,
  mixer: View.Mixer,
  transport: View.Transport,
  timeline: View.Timeline,
  sequencer: View.PatternSequencer,
  'effects-rack': View.EffectsRack,
  'piano-roll': View.PianoRoll,
  // Preferences › Audio Settings…
  audio: View.AudioSettings,
  // File › Mixdown…
  mixdown: View.Mixdown,
  // Dawesome › Preferences… and Preferences › Appearance…
  preferences: View.Preferences,
  appearance: View.Preferences,
  // Preferences › Controls…
  controls: View.Controls,
  // Preferences › File Paths…
  'file-paths': View.FilePaths,
}

// the Window and Preferences menus (and File › Mixdown…): open a tab, or select it where it's already open.
// resolves to a function that stops listening
const viewMenu = (dispatch: AppDispatch) =>
  listen<{ data: string }>('menu-open-view', ev => {
    const view = menuViews[ev.payload.data]
    if(view !== undefined) dispatch(openTab(view))
  })

// the Edit menu's items, by the names the engine sends (see build_menu in
// engine/src/app/window.rs)
export type EditCommand = 'undo' | 'redo' | 'repeat' | 'select-all' | 'join' | 'blade'

// resolves to a function that stops listening
const editMenu = (run: (command: EditCommand) => void) =>
  listen<{ data: EditCommand }>('menu-edit', ev => run(ev.payload.data))

const listeners = { 
  projectMenu,
  recentProjectMenu,
  openedFiles,
  quitRequested,
  viewMenu,
  editMenu,
}

export { listeners }

