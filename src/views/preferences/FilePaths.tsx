import React from 'react'
import { open } from '@tauri-apps/plugin-dialog'
import { useAppDispatch, useAppSelector } from '../../hooks/redux'
import {
  addSamplePath,
  getBrowserRootSamples,
  removeSamplePath,
  selectSamplePaths,
  SamplePath,
} from '../../state/slices/envSlice'
import { selectProjectPath } from '../../state/slices/projectSlice'
import { AnalogButton } from '../../components/fourtrack'
import Notice from '../../components/common/Notice'

const Button = ({ children, ...props }: React.ComponentProps<typeof AnalogButton>) => (
  <AnalogButton variant='text' size='sm' {...props}>{children}</AnalogButton>
)

const errorText = (e: unknown) => (e instanceof Error ? e.message : String(e))

// what a folder that can't be taken out says instead
const KIND_NOTES: Record<SamplePath['kind'], string | null> = {
  bundled: 'The samples Dawesome comes with',
  user: null,
  project: "The open project's samples folder",
}

// Preferences › File Paths: the folders of samples the Samples browser
// shows. the ones Dawesome comes with are always first, then the user's,
// then the open project's samples folder (a shared project's samples)
const FilePaths = () => {
  const dispatch = useAppDispatch()
  const paths = useAppSelector(selectSamplePaths)
  const projectPath = useAppSelector(selectProjectPath)
  const [problem, setProblem] = React.useState<string | null>(null)

  React.useEffect(() => {
    dispatch(getBrowserRootSamples()).catch(e => setProblem(errorText(e)))
  }, [dispatch, projectPath])

  const add = async () => {
    const picked = await open({ directory: true, multiple: false, title: 'Add a folder of samples' })
    if(typeof picked !== 'string') return
    setProblem(null)
    await dispatch(addSamplePath(picked)).catch(e => setProblem(errorText(e)))
  }

  const remove = async (path: string) => {
    setProblem(null)
    await dispatch(removeSamplePath(path)).catch(e => setProblem(errorText(e)))
  }

  return (
    <div className='ft-panel h-full w-full overflow-y-auto p-4 text-sm select-none'>
      <div className='flex flex-col gap-3 max-w-xl'>
        <span className='ft-label'>Folders of samples</span>
        <ul className='flex flex-col gap-2' aria-label='Folders of samples'>
          {paths.map(path => (
            <li
              key={`${path.kind}:${path.path}`}
              className={`flex flex-row items-center gap-3 rounded border border-line px-3 py-2 ${path.found ? '' : 'opacity-60'}`}
            >
              <div className='flex flex-col min-w-0 flex-1'>
                <span className='font-medium truncate'>{path.name}</span>
                <span className='text-xs text-ink-faint truncate select-text' data-hint={path.path}>{path.path}</span>
                {KIND_NOTES[path.kind] && <span className='text-xs text-ink-faint'>{KIND_NOTES[path.kind]}</span>}
                {!path.found && <span className='text-xs text-danger-ink'>Not found</span>}
              </div>
              {path.kind === 'user' && (
                <Button aria-label={`Remove ${path.name}`} onPress={() => { remove(path.path) }}>
                  Remove
                </Button>
              )}
            </li>
          ))}
        </ul>
        <div>
          <Button onPress={() => { add() }}>Add Folder…</Button>
        </div>
        {problem && <Notice tone='danger'>{problem}</Notice>}
        <p className='text-xs text-ink-faint'>
          The Samples browser shows each folder here. Removing a folder only takes it out of the list: nothing on disk changes.
        </p>
      </div>
    </div>
  )
}

export default FilePaths
