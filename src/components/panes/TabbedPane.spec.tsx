import React from 'react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import { act, fireEvent, render, screen } from '@testing-library/react'
import windowReducer, { selectWindowPane } from '../../state/slices/windowSlice'
import { leaves } from '../../state/paneTree'
import { useAppSelector } from '../../hooks/redux'
import { keyOf, tabKey, View } from '../../types/ui'
import TabbedPane from './TabbedPane'

// (the views themselves aren't what's tested)
vi.mock('../../views/matchViewComponent', () => ({ matchViewComponent: () => null }))

const Root = () => {
  const root = useAppSelector(selectWindowPane)
  return <TabbedPane pane={leaves(root)[0]} />
}

const [playlist, mixer, sequencer] = [View.Playlist, View.Mixer, View.PatternSequencer].map(v => tabKey(v))

// (jsdom has none: nothing is under the mouse until a test says)
beforeEach(() => { document.elementFromPoint = () => document.body })

const setup = () => {
  const store = configureStore({ reducer: { window: windowReducer } })
  const { container } = render(<Provider store={store}><Root /></Provider>)
  return { store, container }
}

const tabsOf = (store: ReturnType<typeof setup>['store']) =>
  leaves(store.getState().window.windowPane.root).map(pane => pane.tabs.map(keyOf))

// press on `from`, move, and let go over `target` at (x, y)
const drag = async (from: Element, target: () => Element, x: number, y: number) => {
  fireEvent.mouseDown(from, { button: 0, clientX: 0, clientY: 0 })
  await act(async () => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 20, clientY: 20 })) })
  document.elementFromPoint = () => target()
  await act(async () => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: x, clientY: y })) })
  await act(async () => { window.dispatchEvent(new MouseEvent('mouseup', { clientX: x, clientY: y })) })
}

// the zones shown over a pane's body while a tab is dragged, laid out
// 100 pixels square (jsdom lays nothing out)
const overlay = (container: HTMLElement) => {
  const el = container.querySelector<HTMLElement>('.absolute.inset-0')
  if(!el) throw new Error('no drop zones showing')
  el.getBoundingClientRect = () => ({ left: 0, top: 0, width: 100, height: 100, right: 100, bottom: 100, x: 0, y: 0, toJSON: () => ({}) })
  return el
}

describe('dragging a tab', () => {
  it('splits the pane when let go near an edge of it', async () => {
    const { store, container } = setup()
    await drag(screen.getByText('Mixer'), () => overlay(container), 95, 50)
    const root = store.getState().window.windowPane.root
    expect(root.direction).toBe('vertical')
    expect(tabsOf(store)).toEqual([[playlist, sequencer], [mixer]])
  })

  it('moves the tab to the end when let go after the tabs', async () => {
    const { store } = setup()
    const end = () => screen.getByText('Pattern Sequencer').closest('[role="tablist"]')?.nextElementSibling as Element
    await drag(screen.getByText('Playlist'), end, 10, 10)
    expect(tabsOf(store)).toEqual([[mixer, sequencer, playlist]])
  })

  it('shows the drop zones only while a tab is dragged', async () => {
    const { container } = setup()
    expect(container.querySelector('.absolute.inset-0')).toBeNull()
    fireEvent.mouseDown(screen.getByText('Mixer'), { button: 0, clientX: 0, clientY: 0 })
    await act(async () => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 20, clientY: 20 })) })
    expect(container.querySelector('.absolute.inset-0')).toBeTruthy()
    await act(async () => { window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' })) })
    expect(container.querySelector('.absolute.inset-0')).toBeNull()
  })

  it('isn’t started from a tab’s close button', async () => {
    const { store, container } = setup()
    fireEvent.mouseDown(screen.getByRole('button', { name: 'Close Mixer' }), { button: 0, clientX: 0, clientY: 0 })
    await act(async () => { window.dispatchEvent(new MouseEvent('mousemove', { clientX: 20, clientY: 20 })) })
    expect(container.querySelector('.absolute.inset-0')).toBeNull()
    await act(async () => { window.dispatchEvent(new MouseEvent('mouseup', { clientX: 20, clientY: 20 })) })
    expect(tabsOf(store)).toEqual([[playlist, mixer, sequencer]])
  })
})
