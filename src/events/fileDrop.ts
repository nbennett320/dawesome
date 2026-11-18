import { invoke } from '@tauri-apps/api/core'
import type { DroppedFile, SampleDragEvent, SkippedFile } from '../state/sampleDrag'

// files dragged in from outside the app (from Finder), passed on as drags
// like the sidebar's (see sampleDrag.ts), so the playlist, the Clips page
// and the Pattern Sequencer take them where they take samples
//
// the window hears of them from Tauri (dragDropEnabled in tauri.conf.json):
// enter (with the paths), over, drop (with the paths) and leave. what was
// dragged in is scanned by the engine as it enters, for the files in it that
// can be played; until that's answered, moves wait

// what the engine makes of what's dragged in (scan_dropped_files)
export interface DropScan {
  files: DroppedFile[]
  skipped: SkippedFile[]
}

// a point Tauri gives, in the window's CSS pixels. on macOS it's already in
// points (wry hands them on as they are); elsewhere it's in device pixels
export const toClient = (
  position: { x: number, y: number },
  mac: boolean,
  pixelRatio: number,
): { clientX: number, clientY: number } => {
  const scale = mac ? 1 : pixelRatio || 1
  return { clientX: position.x / scale, clientY: position.y / scale }
}

export type DropPayload =
  | { type: 'enter', paths: string[], position: { x: number, y: number } }
  | { type: 'over', position: { x: number, y: number } }
  | { type: 'drop', paths: string[], position: { x: number, y: number } }
  | { type: 'leave' }

interface Options {
  scan: (paths: string[]) => Promise<DropScan>
  send: (ev: SampleDragEvent) => void
  toClient: (position: { x: number, y: number }) => { clientX: number, clientY: number }
}

// follows one drag in from outside at a time, and passes it on. returns
// what to call with each of Tauri's events
export const fileDropFollower = ({ scan, send, toClient: client }: Options) => {
  // the drag in progress: its scan, once answered, and where it is now
  let drag: { scanned: Promise<DropScan>, scan: DropScan | null, at: ReturnType<typeof client>, gone: boolean } | null = null

  const item = (found: DropScan) => ({ kind: 'files' as const, files: found.files, skipped: found.skipped })

  return async (payload: DropPayload) => {
    if(payload.type === 'enter') {
      const scanned = scan(payload.paths).catch((): DropScan => ({
        files: [], skipped: payload.paths.map(path => ({ path, reason: 'could not be read' })),
      }))
      const now = { scanned, scan: null as DropScan | null, at: client(payload.position), gone: false }
      drag = now
      now.scan = await scanned
      // (still the drag that's in: show it where the mouse is now)
      if(drag === now && !now.gone) send({ type: 'move', item: item(now.scan), ...now.at })
      return
    }

    const now = drag
    if(!now) return

    if(payload.type === 'over') {
      now.at = client(payload.position)
      if(now.scan) send({ type: 'move', item: item(now.scan), ...now.at })
      return
    }

    drag = null
    now.gone = true
    if(payload.type === 'leave') {
      if(now.scan) send({ type: 'cancel', item: item(now.scan), ...now.at })
      return
    }

    // dropped: once it's been scanned
    const found = now.scan ?? await now.scanned
    send({ type: 'drop', item: item(found), ...client(payload.position) })
  }
}

// start passing on drags from outside the window. resolves to a function that
// stops listening
export const listenForFileDrops = async () => {
  const [{ getCurrentWebview }, { sendSampleDrag }] = await Promise.all([
    import('@tauri-apps/api/webview'),
    import('../state/sampleDrag'),
  ])
  const mac = /Mac/.test(navigator.userAgent)
  const follow = fileDropFollower({
    scan: paths => invoke<DropScan>('scan_dropped_files', { paths }),
    send: sendSampleDrag,
    toClient: position => toClient(position, mac, window.devicePixelRatio),
  })
  return getCurrentWebview().onDragDropEvent(ev => {
    // eslint-disable-next-line no-console
    follow(ev.payload).catch(e => console.error('could not follow a drag in:', e))
  })
}
