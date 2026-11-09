import reducer, { pressSidebarPage, setSidebar, WindowState } from './windowSlice'
import { SidebarPage } from '../../types/ui'

const initial = (): WindowState => reducer(undefined, { type: '@@init' })

describe('pressSidebarPage', () => {
  it('closes the sidebar when the page showing is pressed', () => {
    const state = reducer(initial(), pressSidebarPage(SidebarPage.Samples))
    expect(state.sidebar).toBe(false)
    expect(state.sidebarPage).toBe(SidebarPage.Samples)
  })

  it('switches page while open', () => {
    const state = reducer(initial(), pressSidebarPage(SidebarPage.Patterns))
    expect(state.sidebar).toBe(true)
    expect(state.sidebarPage).toBe(SidebarPage.Patterns)
  })

  it('opens on the page pressed while closed, not the one it was on', () => {
    const closed = reducer(initial(), setSidebar(false))
    const state = reducer(closed, pressSidebarPage(SidebarPage.Recordings))
    expect(state.sidebar).toBe(true)
    expect(state.sidebarPage).toBe(SidebarPage.Recordings)
  })

  it('opens again on the same page after closing it', () => {
    const closed = reducer(initial(), pressSidebarPage(SidebarPage.Samples))
    const state = reducer(closed, pressSidebarPage(SidebarPage.Samples))
    expect(state.sidebar).toBe(true)
    expect(state.sidebarPage).toBe(SidebarPage.Samples)
  })
})
