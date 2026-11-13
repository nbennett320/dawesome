import { createSlice, Dispatch } from '@reduxjs/toolkit'
import { invoke } from '@tauri-apps/api/core'
import { ask, message, open, save } from '@tauri-apps/plugin-dialog'
import { readDir } from '@tauri-apps/plugin-fs'
import { dirname, documentDir, join } from '@tauri-apps/api/path'
import type { RootState } from 'state/store'
import { MissingEffect, MissingSample, PROJECT_EXTENSION } from '../../types/project'
import { SynthInstrument } from '../../types/sequencer'
import { newProject, openProject, saveProject } from './playlistSlice'
import { clearPendingMixerSends } from './mixerSlice'
import { setSequencer } from './sequencerSlice'
import { dropHeldTrim } from '../pendingTrim'
import { fileName } from '../../util/helpers'

// the File menu's project commands (ids of the engine's menu items, see
// engine/src/app/window.rs)
export type ProjectCommand = 'new' | 'open' | 'save' | 'save-as'

// shown above the workspace after a project command
export type ProjectNotice =
  // what a project couldn't find, or left out. (older notices may not have
  // the synths and what was dropped)
  | { kind: 'missing', samples: MissingSample[], effects: MissingEffect[], synths?: SynthInstrument[], dropped?: string[] }
  | { kind: 'error', message: string }
  // something worth knowing, like where a take was saved
  | { kind: 'info', message: string }

export interface ProjectState {
  // the project file being worked on, or null before it's first saved
  path: string | null
  notice: ProjectNotice | null
  // whether the project has changes that aren't saved, as the engine last
  // said (see src/state/unsaved.ts). shown as a * in the window's title
  unsaved: boolean
  // counts the times the project was saved, opened or started, which leave
  // it with nothing unsaved: a check that began before one is out of date
  settled: number
}

const initialState: ProjectState = {
  path: null,
  notice: null,
  unsaved: false,
  settled: 0,
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
    setUnsaved: (state, action: { type: string, payload: boolean }) => {
      state.unsaved = action.payload
    },
    // the project was just saved, opened or started: nothing's unsaved
    settleProject: state => {
      state.unsaved = false
      state.settled += 1
    },
  },
})

export const { setProjectPath, setProjectNotice, setUnsaved, settleProject } = projectSlice.actions

export const selectProjectPath = (state: RootState) => state.project.path
export const selectProjectNotice = (state: RootState) => state.project.notice
export const selectProjectUnsaved = (state: RootState) => state.project.unsaved

// the project's name, from its file name, or null before it's saved
export const projectName = (path: string | null): string | null => {
  if(!path) return null

  const file = path.split(/[\\/]/).pop() ?? path
  return file.replace(new RegExp(`\\.${PROJECT_EXTENSION}$`, 'i'), '')
}

// the window's title: the project's name, with a * while it has unsaved
// changes
export const windowTitle = (path: string | null, unsaved: boolean): string =>
  `${projectName(path) ?? 'Untitled'}${unsaved ? '*' : ''} — Dawesome`

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

// whether it's fine to replace the project: it has no unsaved changes (see
// has_unsaved_changes in engine/src/project.rs), or the user says so
const okToReplace = async (): Promise<boolean> => {
  // if the engine can't say, ask: better than losing changes
  const changed = await invoke<boolean>('project_has_unsaved_changes').catch(() => true)
  if(!changed) return true

  return ask(
    'Your unsaved changes to the current project will be lost.',
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

// open a project file, showing any samples or plugins it couldn't find. returns
// whether it opened
const openFrom = async (dispatch: Dispatch, path: string): Promise<boolean> => {
  try {
    // fader moves not sent yet, and a trim held for the play cursor,
    // belong to the old project
    clearPendingMixerSends()
    dropHeldTrim()
    const missing = await openProject(path)(dispatch)
    const { samples, effects, synths, dropped } = missing
    dispatch(setProjectPath(path))
    dispatch(settleProject())
    dispatch(setProjectNotice(
      [samples, effects, synths, dropped].some(list => list.length > 0) ? { kind: 'missing', ...missing } : null,
    ))
    await noteRecent(path)
    return true
  } catch(e) {
    dispatch(setProjectNotice({ kind: 'error', message: errorMessage("Couldn't open the project", e) }))
    return false
  }
}

// save the project, and carry on in the file it was saved to. returns
// whether it saved
// - somewhereNew: a first save or Save As, which gets a folder of its own
//   (the engine says where that put it). otherwise saved right at `path`
const saveProjectTo = async (
  dispatch: Dispatch,
  getState: () => RootState,
  path: string,
  somewhereNew: boolean,
): Promise<boolean> => {
  let saved
  try {
    saved = await saveProject(path, somewhereNew)(dispatch, getState)
  } catch(e) {
    dispatch(setProjectNotice({ kind: 'error', message: errorMessage("Couldn't save the project", e) }))
    return false
  }

  dispatch(setProjectPath(saved.path))
  dispatch(settleProject())
  if(saved.sequencer) dispatch(setSequencer(saved.sequencer))
  dispatch(setProjectNotice(saved.problems.length > 0
    ? { kind: 'error', message: `Saved, but some takes are still outside the project's folder: ${saved.problems.join('; ')}` }
    : null))
  await noteRecent(saved.path)
  return true
}

// File › Save As: ask where, then save there. returns whether it saved (not
// if the dialog was cancelled)
const saveAs = async (dispatch: Dispatch, getState: () => RootState): Promise<boolean> => {
  const chosen = chosenPath(await save({
    title: 'Save project',
    filters: dialogFilters,
    defaultPath: await saveDialogStart(getState().project.path),
  }))

  return chosen ? saveProjectTo(dispatch, getState, withExtension(chosen), true) : false
}

// File › Save: where it was saved last, or (never saved) where the user
// chooses. returns whether it saved
const saveCurrent = (dispatch: Dispatch, getState: () => RootState): Promise<boolean> => {
  const current = getState().project.path
  return current ? saveProjectTo(dispatch, getState, current, false) : saveAs(dispatch, getState)
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
  switch(command) {
    case 'new': {
      if(!(await okToReplace())) break

      try {
        clearPendingMixerSends()
        dropHeldTrim()
        await newProject()(dispatch)
        dispatch(setProjectPath(null))
        dispatch(settleProject())
        dispatch(setProjectNotice(null))
      } catch(e) {
        dispatch(setProjectNotice({ kind: 'error', message: errorMessage("Couldn't start a new project", e) }))
      }
      break
    }
    case 'open': {
      if(!(await okToReplace())) break

      const chosen = chosenPath(await open({
        title: 'Open project',
        filters: dialogFilters,
        multiple: false,
      }))
      if(chosen) await openFrom(dispatch, chosen)
      break
    }
    case 'save':
      await saveCurrent(dispatch, getState)
      break
    case 'save-as':
      await saveAs(dispatch, getState)
      break
    default:
      break
  }
})

// open a project from File › Open Recent. one that can't be opened (moved,
// deleted, or broken) is taken off the list
export const openRecentProject = (path: string) => (
  dispatch: Dispatch,
): Promise<void> => exclusive(async () => {
  if(!(await okToReplace())) return

  if(!(await openFrom(dispatch, path))) await forgetRecent(path)
})

// the engine was asked to quit (Quit, the window's close button, the
// system) with changes that aren't saved: ask whether to save them first.
// the engine quits when it's told to (see may_quit in
// engine/src/app/window.rs)
export const confirmQuit = () => (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<void> => exclusive(async () => {
  const name = projectName(getState().project.path) ?? 'Untitled'
  const choice = await message(
    "Your changes will be lost if you don't save them.",
    {
      title: `Save changes to "${name}" before quitting?`,
      kind: 'warning',
      buttons: { yes: 'Save', no: "Don't Save", cancel: 'Cancel' },
    },
  )

  if(choice === 'Save' || choice === 'Yes') {
    // not saved (the dialog cancelled, or it failed): carry on, with the
    // notice saying why
    if(!(await saveCurrent(dispatch, getState))) return
  } else if(choice !== "Don't Save" && choice !== 'No') {
    return
  }
  await invoke('quit')
})

// the command line's :qa, as in vim: quit, but only with nothing unsaved.
// resolves to why it didn't, if it didn't
export const quitIfSaved = () => async (): Promise<string | undefined> => {
  // if the engine can't say, don't: better than losing changes
  const changed = await invoke<boolean>('project_has_unsaved_changes').catch(() => true)
  if(changed) return 'Unsaved changes (add ! to quit anyway, or :wqa to save and quit)'
  await invoke('quit')
  return undefined
}

// the command line's :wqa: save (asking where, if it's never been saved),
// then quit. stays open if it wasn't saved, with the notice saying why
export const saveAndQuit = () => (
  dispatch: Dispatch,
  getState: () => RootState,
): Promise<void> => exclusive(async () => {
  if(await saveCurrent(dispatch, getState)) await invoke('quit')
})

export default projectSlice.reducer
