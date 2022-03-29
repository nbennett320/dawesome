import React from 'react'
import useSwr from 'swr'
import { invoke } from '@tauri-apps/api'

const invokeFetcher = async <TArgs extends Record<string, any>, TResult>(
  command: string,
  args: TArgs
): Promise<TResult> => invoke<TResult>(command, args)

export const useInvoke = <TArgs extends Record<string, any>, TResult>(
  payload: TArgs,
  getCommand: string,
  setCommand: string
) => {
  const { data, error, mutate } = useSwr<TResult>(
    [getCommand, payload],
    invokeFetcher
  )

  const update = React.useCallback(
    async (newData: TResult) => {
      await invoke(setCommand, { ...payload })
      mutate()
    },
    [payload, mutate, setCommand]
  )

  return {
    data,
    fetching: !data,
    error,
    update,
  }
}
