import { DropScan, fileDropFollower, toClient } from './fileDrop'
import type { SampleDragEvent } from '../state/sampleDrag'

const scanned: DropScan = {
  files: [{ path: '/a/kick.wav', folder: '' }],
  skipped: [{ path: '/a/notes.txt', reason: 'not audio' }],
}
const item = { kind: 'files', files: scanned.files, skipped: scanned.skipped }

// a follower whose scan answers when `answer` is called
const setup = () => {
  const sent: SampleDragEvent[] = []
  let answer: (scan: DropScan) => void = () => {}
  const scan = vi.fn(() => new Promise<DropScan>(resolve => { answer = resolve }))
  const follow = fileDropFollower({
    scan,
    send: ev => { sent.push(ev) },
    toClient: ({ x, y }) => ({ clientX: x / 2, clientY: y / 2 }),
  })
  return { sent, scan, follow, answer: (s: DropScan) => answer(s) }
}

describe('toClient', () => {
  it('takes macOS points as they are, and scales device pixels elsewhere', () => {
    expect(toClient({ x: 200, y: 100 }, true, 2)).toEqual({ clientX: 200, clientY: 100 })
    expect(toClient({ x: 200, y: 100 }, false, 2)).toEqual({ clientX: 100, clientY: 50 })
    expect(toClient({ x: 200, y: 100 }, false, 0)).toEqual({ clientX: 200, clientY: 100 })
  })
})

describe('fileDropFollower', () => {
  it('scans what enters, holding moves until the scan answers', async () => {
    const { sent, scan, follow, answer } = setup()
    const entered = follow({ type: 'enter', paths: ['/a/kick.wav', '/a/notes.txt'], position: { x: 10, y: 20 } })
    expect(scan).toHaveBeenCalledWith(['/a/kick.wav', '/a/notes.txt'])
    await follow({ type: 'over', position: { x: 30, y: 40 } })
    expect(sent).toEqual([])

    answer(scanned)
    await entered
    // shown where the mouse is now
    expect(sent).toEqual([{ type: 'move', item, clientX: 15, clientY: 20 }])
    await follow({ type: 'over', position: { x: 50, y: 60 } })
    expect(sent[1]).toEqual({ type: 'move', item, clientX: 25, clientY: 30 })
  })

  it('drops where it was let go', async () => {
    const { sent, follow, answer } = setup()
    const entered = follow({ type: 'enter', paths: ['/a/kick.wav'], position: { x: 10, y: 20 } })
    answer(scanned)
    await entered
    await follow({ type: 'drop', paths: ['/a/kick.wav'], position: { x: 80, y: 90 } })
    expect(sent.at(-1)).toEqual({ type: 'drop', item, clientX: 40, clientY: 45 })
    // it's over: later moves go nowhere
    await follow({ type: 'over', position: { x: 1, y: 1 } })
    expect(sent.at(-1)?.type).toBe('drop')
  })

  it('a drop before the scan answers waits for it, and shows no move after', async () => {
    const { sent, follow, answer } = setup()
    const entered = follow({ type: 'enter', paths: ['/a/kick.wav'], position: { x: 10, y: 20 } })
    const dropped = follow({ type: 'drop', paths: ['/a/kick.wav'], position: { x: 80, y: 90 } })
    answer(scanned)
    await Promise.all([entered, dropped])
    expect(sent).toEqual([{ type: 'drop', item, clientX: 40, clientY: 45 }])
  })

  it('cancels when it leaves the window', async () => {
    const { sent, follow, answer } = setup()
    const entered = follow({ type: 'enter', paths: ['/a/kick.wav'], position: { x: 10, y: 20 } })
    answer(scanned)
    await entered
    await follow({ type: 'leave' })
    expect(sent.at(-1)).toEqual({ type: 'cancel', item, clientX: 5, clientY: 10 })
  })

  it('a scan that fails leaves everything out, as unreadable', async () => {
    const sent: SampleDragEvent[] = []
    const follow = fileDropFollower({
      scan: () => Promise.reject(new Error('no')),
      send: ev => { sent.push(ev) },
      toClient: ({ x, y }) => ({ clientX: x, clientY: y }),
    })
    await follow({ type: 'enter', paths: ['/a/x.wav'], position: { x: 1, y: 2 } })
    expect(sent).toEqual([{
      type: 'move',
      item: { kind: 'files', files: [], skipped: [{ path: '/a/x.wav', reason: 'could not be read' }] },
      clientX: 1,
      clientY: 2,
    }])
  })
})
