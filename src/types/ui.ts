import React from 'react'

export enum TabTypes {
  Tab = 'tab',
}

export type SplitDirection = 'vertical' | 'horizontal'

export type PaneTab = {
  label: string
  index: number
  active: boolean
  component: React.ReactChild
}

export type WindowNode = {
  id: string
  left?: WindowNode
  right?: WindowNode
  child?: React.ReactChild
  tabs: Array<PaneTab>
  direction: SplitDirection
}

export type WindowPane = {
  id: string
  root: WindowNode
}
