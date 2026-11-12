import React from 'react'
import { useAppDispatch } from '../../../hooks/redux'
import { setProjectNotice } from '../../../state/slices/projectSlice'

const message = (e: unknown) => (e instanceof Error ? e.message : String(e))

// tells the user that something couldn't be done: `.catch(fail('move the folder'))`
export const useFail = () => {
  const dispatch = useAppDispatch()
  return React.useCallback((what: string) => (e: unknown) => {
    dispatch(setProjectNotice({ kind: 'error', message: `Couldn't ${what}: ${message(e)}` }))
  }, [dispatch])
}
