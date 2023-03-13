import React from 'react'

export enum TabTypes {
  Tab = 'tab',
}

export enum View {
  Playlist,
  Test
}

export type SplitDirection = 'vertical' | 'horizontal'

export type PaneTab = {
  label: string
  index: number
  active: boolean
  component: View
}

export type WindowNode = {
  id: string
  left?: WindowNode
  right?: WindowNode
  child?: View
  tabs: Array<PaneTab>
  direction: SplitDirection
}

export type WindowPane = {
  id: string
  root: WindowNode
}
