import { SequencerState } from './sequencer'
import { FolderInfo, ROOT } from './folderTree'

// the project's recorded takes, as the sidebar's recordings browser shows
// them. mirrors RecordingFile in engine/src/project_files.rs

export interface RecordingFile {
  path: string
  // the file name, without .wav
  name: string
  // the folder it's in, relative to the recordings folder ('' at the top,
  // / between folders)
  folder: string
  // how long it plays, or null when the engine couldn't read it (e.g. it's
  // still being recorded)
  durationSecs: number | null
  // when it was last written, in ms since the unix epoch
  modifiedMs: number | null
}

// what's in the recordings folder (see t_list_recordings)
export interface RecordingsListing {
  // newest first
  takes: RecordingFile[]
  // every folder, relative to the recordings folder, empty ones too
  folders: string[]
}

// where a take is used
export interface TakeUse {
  clips: number
  rows: number
}

// a take's length: 0:07, 1:42, or 1:02:03 past an hour
export const formatTakeDuration = (secs: number | null): string => {
  if(secs === null || !Number.isFinite(secs) || secs < 0) return '–'

  const total = Math.round(secs)
  const s = total % 60
  const m = Math.floor(total / 60) % 60
  const h = Math.floor(total / 3600)
  const ss = String(s).padStart(2, '0')
  return h ? `${h}:${String(m).padStart(2, '0')}:${ss}` : `${m}:${ss}`
}

const sameDay = (a: Date, b: Date) =>
  a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()

// when a take was recorded, short enough for its row: the time today, the
// day this year, or the date
export const formatTakeDate = (ms: number | null, now = new Date()): string => {
  if(ms === null) return ''

  const date = new Date(ms)
  if(sameDay(date, now)) {
    return date.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })
  }
  if(date.getFullYear() === now.getFullYear()) {
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
  }
  return date.toLocaleDateString(undefined, { year: '2-digit', month: 'numeric', day: 'numeric' })
}

// how many playlist clips and sequencer rows use each take, by path
export const takeUses = (
  clipPaths: string[],
  rowPaths: string[],
): Map<string, TakeUse> => {
  const uses = new Map<string, TakeUse>()
  const use = (path: string) => {
    let found = uses.get(path)
    if(!found) {
      found = { clips: 0, rows: 0 }
      uses.set(path, found)
    }
    return found
  }
  clipPaths.forEach(path => { if(path) use(path).clips += 1 })
  rowPaths.forEach(path => { if(path) use(path).rows += 1 })
  return uses
}

// says where a take is used, for its tooltip
export const describeUse = (use: TakeUse | undefined): string => {
  if(!use || (!use.clips && !use.rows)) return 'Not used in the project'

  const parts: string[] = []
  if(use.clips) parts.push(`${use.clips} clip${use.clips === 1 ? '' : 's'} in the playlist`)
  if(use.rows) parts.push(`${use.rows} Sequencer row${use.rows === 1 ? '' : 's'}`)
  return `Used by ${parts.join(' and ')}`
}

// the takes whose name has the search in it
export const searchTakes = (takes: RecordingFile[], query: string): RecordingFile[] => {
  const search = query.trim().toLowerCase()
  if(!search) return takes
  return takes.filter(take => take.name.toLowerCase().includes(search))
}

// a take or folder after it's moved or renamed (see MovedRecording in
// engine/src/audio_commands.rs), and the clips and rows that now play from
// its new place
export interface MovedRecording {
  // a take's full path, or a folder's path relative to the recordings folder
  path: string
  movedClips: Array<{ id: number, path: string }>
  sequencer: SequencerState | null
  // the saved project file couldn't follow it: it'll find it once saved
  problem: string | null
}

// a recordings folder's path (relative to the recordings folder, / between
// folders) as a folder in the browser's tree: the path is its key
export const recordingFolders = (paths: string[]): FolderInfo[] => paths.map(path => {
  const cut = path.lastIndexOf('/')
  return { key: path, name: path.slice(cut + 1), parent: cut < 0 ? ROOT : path.slice(0, cut) }
})

// the folder a folder is in
export const parentFolder = (path: string): string => {
  const cut = path.lastIndexOf('/')
  return cut < 0 ? ROOT : path.slice(0, cut)
}

// a folder path after the folder `from` moved to `to` (it, or one inside
// it), or the path as it was
export const movedFolderPath = (path: string, from: string, to: string): string => {
  if(path === from) return to
  if(path.startsWith(`${from}/`)) return to + path.slice(from.length)
  return path
}
