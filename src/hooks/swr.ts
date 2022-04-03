import React from 'react'
import useSwr from 'swr'
import { invoke } from '@tauri-apps/api'

interface TResult extends Record<string, any> {
  data?: Record<string, any>
}

interface Response {
  data: TResult | undefined,
  fetching: boolean,
  error: any,
  update: (d: any) => Promise<void>
}

const invokeFetcher = async <TArgs extends Record<string, any>, TResult>(
  command: string,
  args: TArgs
): Promise<TResult> => invoke<TResult>(command, args)

export const useInvoke = <TArgs extends Record<string, any>, TResult>(
  args: TArgs,
  getCommand: string,
  setCommand: string
): Response => {
  const { data, error, mutate } = useSwr<Record<string, any>>(
    [getCommand, args],
    invokeFetcher
  )

  const update = React.useCallback(
    async (newData: TResult) => {
      await invoke(setCommand, { ...args })
      mutate()
    },
    [args, mutate, setCommand]
  )

  return {
    data: data,
    fetching: !data,
    error,
    update,
  }
}
