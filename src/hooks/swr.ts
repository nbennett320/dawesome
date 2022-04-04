import React from 'react'
import useSwr from 'swr'
import { invoke } from '@tauri-apps/api'

interface TResult extends Record<string, unknown> {
  data?: Record<string, unknown>
}

interface Response {
  data: TResult | undefined
  fetching: boolean
  error: unknown
  update: (d: unknown) => Promise<void>
}

const invokeFetcher = async <TArgs extends Record<string, unknown>>(command: string, args: TArgs): Promise<TResult> =>
  invoke<TResult>(command, args)

export const useInvoke = <TArgs extends Record<string, unknown>>(
  args: TArgs,
  getCommand: string,
  setCommand: string,
): Response => {
  const { data, error, mutate } = useSwr<Record<string, unknown>>([getCommand, args], invokeFetcher)

  const update = React.useCallback(async () => {
    await invoke(setCommand, { ...args })
    mutate()
  }, [args, mutate, setCommand])

  return {
    data,
    fetching: !data,
    error,
    update,
  }
}
