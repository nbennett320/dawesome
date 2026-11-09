import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api'
import { ask, open, save } from '@tauri-apps/api/dialog'
import { readDir } from '@tauri-apps/api/fs'
import { dirname, documentDir, join } from '@tauri-apps/api/path'
import type { RootState } from 'state/store'
import { MissingSample, PROJECT_EXTENSION } from '../../types/project'
import { newProject, openProject, saveProject } from './playlistSlice'
import { clearPendingMixerSends } from './mixerSlice'
import { setSequencer } from './sequencerSlice'

// the File menu's project commands (ids of the engine's menu items, see
// engine/src/app/window.rs)
export type ProjectCommand = 'new' | 'open' | 'save' | 'save-as'

// shown above the workspace after a project command
export type ProjectNotice =
  | { kind: 'missing', samples: MissingSample[] }
  | { kind: 'error', message: string }
  // something worth knowing, like where a take was saved
  | { kind: 'info', message: string }

export interface ProjectState {
  // the project file being worked on, or null before it's first saved
  path: string | null
  notice: ProjectNotice | null
}

const initialState: ProjectState = {
  path: null,
  notice: null,
}

export const projectSlice = createSlice({
  name: 'projectSlice',
  initialState,
  reducers: {
    setProjectPath: (state, action: { type: string, payload: string | null }) => {
      state.path = action.payload
    },
    setProjectNotice: (state, action: { type: string, payload: ProjectNotice | null }) => {
      state.notice = action.payload
    },
  },
})

export const { setProjectPath, setProjectNotice } = projectSlice.actions

export const selectProjectPath = (state: RootState) => state.project.path
export const selectProjectNotice = (state: RootState) => state.project.notice

// the project's name, from its file name, or null before it's saved
export const projectName = (path: string | null): string | null => {
  if(!path) return null

  const file = path.split(/[\\/]/).pop() ?? path
  return file.replace(new RegExp(`\\.${PROJECT_EXTENSION}$`, 'i'), '')
}

const dialogFilters = [{ name: 'Dawesome project', extensions: [PROJECT_EXTENSION] }]

// a dialog's chosen path, or null if it was cancelled
const chosenPath = (chosen: string | string[] | null | undefined): string | null => {
  if(Array.isArray(chosen)) return chosen[0] ?? null
  return chosen || null
}

// where the save dialog starts: the project's file, or an untitled project in
// the documents folder
//
// it must be a full path. given just a file name, the dialog is told to start
// in the folder "", which crashes the app on macOS (Tauri 1.0-rc.6, rfd 0.8)
const isAbsolute = (path: string) => path.startsWith('/') || /^[a-z]:[\\/]/i.test(path)

const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path

// the folder a project is in. a project in a folder of its own (named after
// it) counts as being in the folder around that, so saving next to it
// doesn't put one project inside another
const projectHome = async (path: string): Promise<string> => {
  const folder = await dirname(path)
  return fileName(folder) === projectName(path) ? dirname(folder) : folder
}

const saveDialogStart = async (current: string | null, name = 'Untitled'): Promise<string | undefined> => {
  if(current && isAbsolute(current)) return join(await projectHome(current), fileName(current))

  try {
    const start = await join(await documentDir(), `${name}.${PROJECT_EXTENSION}`)
    return isAbsolute(start) ? start : undefined
  } catch {
    return undefined
  }
}

const withExtension = (path: string) =>
  (path.toLowerCase().endsWith(`.${PROJECT_EXTENSION}`) ? path : `${path}.${PROJECT_EXTENSION}`)

// whether it's fine to replace what's in the playlist
// (until changes are tracked, this asks whenever the playlist has clips)
const okToReplace = async (getState: () => RootState): Promise<boolean> => {
  if(getState().playlist.playlistItems.length === 0) return true

  return ask(
    'Any unsaved changes to the current project will be lost.',
    'Replace the current project?',
  )
}

const errorMessage = (what: string, e: unknown) =>
  `${what}: ${e instanceof Error ? e.message : String(e)}`

// put a project at the top of File › Open Recent. failing to is only logged,
// since the project itself was opened or saved fine
const noteRecent = (path: string) =>
  invoke<void>('add_recent_project', { path })
    // eslint-disable-next-line no-console
    .catch(e => console.error('could not update recent projects:', e))

const forgetRecent = (path: string) =>
  invoke<void>('remove_recent_project', { path })
    // eslint-disable-next-line no-console
    .catch(e => console.error('could not update recent projects:', e))

// one command at a time, so pressing a shortcut twice doesn't open two dialogs
let running = false

const exclusive = async (run: () => Promise<void>) => {
  if(running) return
  running = true

  try {
    await run()
  } finally {
    running = false
  }
}

// open a project file, showing any samples it couldn't find. returns
// whether it opened
const openFrom = async (dispatch: Dispatch, path: string): Promise<boolean> => {
  try {
    // fader moves not sent yet belong to the old project
    clearPendingMixerSends()
    const missing = await openProject(path)(dispatch)
    dispatch(setProjectPath(path))
    dispatch(setProjectNotice(missing.length > 0 ? { kind: 'missing', samples: missing } : null))
    await noteRecent(path)
    return true
  } catch(e) {
    dispatch(setProjectNotice({ kind: 'error', message: errorMessage("Couldn't open the project", e) }))
    return false
  }
}

// save the project, and carry on in the file it was saved to
// - somewhereNew: a first save or Save As, which gets a folder of its own
//   (the engine says where that put it). otherwise saved right at `path`
const saveProjectTo = async (
  dispatch: Dispatch,
  getState: () => RootState,
  path: string,
  somewhereNew: boolean,
) => {
  let saved
  try {
    saved = await saveProject(path, somewhereNew)(dispatch, getState)
  } catch(e) {
    dispatch(setProjectNotice({ kind: 'error', message: errorMessage("Couldn't save the project", e) }))
    return
  }

  dispatch(setProjectPath(saved.path))
  if(saved.sequencer) dispatch(setSequencer(saved.sequencer))
  dispatch(setProjectNotice(saved.problems.length > 0
    ? { kind: 'error', message: `Saved, but some takes are still outside the project's folder: ${saved.problems.join('; ')}` }
    : null))
  await noteRecent(saved.path)
}

// a typed project name, kept to what's fine in a file name: no extension,
// no path separators or characters some systems refuse
export const cleanProjectName = (typed: string): string =>
  typed
    .trim()
    .replace(new RegExp(`\\.${PROJECT_EXTENSION}$`, 'i'), '')
    .replace(/[\\/:*?"<>|]/g, '')
    .trim()

// whether a folder already has a file of this name (case doesn't matter, as
// on macOS and Windows). a folder that can't be read counts as not
const fileExists = async (folder: string, file: string): Promise<boolean> => {
  try {
    const entries = await readDir(folder)
    return entries.some(entry => entry.name?.toLowerCase() === file.toLowerCase())
  } catch {
    return false
  }
}

// give the project a name, from the cassette's label
// - an unsaved project: the save dialog, with the name filled in
// - a saved one: after asking, a copy saved as the new name next to it,
//   which is then the project's file. the old file is left as it was
export const nameProject = (typed: string) => (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<void> => exclusive(async () => {
  const name = cleanProjectName(typed)
  const current = getState().project.path
  if(!name || name === projectName(current)) return

  if(!current) {
    const chosen = chosenPath(await save({
      title: 'Save project',
      filters: dialogFilters,
      defaultPath: await saveDialogStart(null, name),
    }))
    if(chosen) await saveProjectTo(dispatch, getState, withExtension(chosen), true)
    return
  }

  // in a folder of its own, next to the project's folder
  const chosen = await join(await projectHome(current), `${name}.${PROJECT_EXTENSION}`)
  const target = await invoke<string>('project_file_for', { path: chosen })
  const replacing = await fileExists(await dirname(target), fileName(target))
  const was = projectName(current)

  const ok = await ask(
    replacing
      ? `A project called "${name}" is already in this folder, and will be replaced. "${was}" stays as it was.`
      : `The project will be saved as "${name}", next to "${was}", which stays as it was.`,
    `Save as "${name}"?`,
  )
  if(ok) await saveProjectTo(dispatch, getState, chosen, true)
})

// run a File menu command, asking where to open or save when needed
export const runProjectCommand = (command: ProjectCommand) => (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<void> => exclusive(async () => {

  const saveAs = async () => {
    const chosen = chosenPath(await save({
      title: 'Save project',
      filters: dialogFilters,
      defaultPath: await saveDialogStart(getState().project.path),
    }))

    if(chosen) await saveProjectTo(dispatch, getState, withExtension(chosen), true)
  }

  switch(command) {
    case 'new': {
      if(!(await okToReplace(getState))) break

      try {
        clearPendingMixerSends()
        await newProject()(dispatch)
        dispatch(setProjectPath(null))
        dispatch(setProjectNotice(null))
      } catch(e) {
        dispatch(setProjectNotice({ kind: 'error', message: errorMessage("Couldn't start a new project", e) }))
      }
      break
    }
    case 'open': {
      if(!(await okToReplace(getState))) break

      const chosen = chosenPath(await open({
        title: 'Open project',
        filters: dialogFilters,
        multiple: false,
      }))
      if(chosen) await openFrom(dispatch, chosen)
      break
    }
    case 'save': {
      const current = getState().project.path
      if(current) await saveProjectTo(dispatch, getState, current, false)
      else await saveAs()
      break
    }
    case 'save-as':
      await saveAs()
      break
    default:
      break
  }
})

// open a project from File › Open Recent. one that can't be opened (moved,
// deleted, or broken) is taken off the list
export const openRecentProject = (path: string) => (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<void> => exclusive(async () => {
  if(!(await okToReplace(getState))) return

  if(!(await openFrom(dispatch, path))) await forgetRecent(path)
})

export default projectSlice.reducer
