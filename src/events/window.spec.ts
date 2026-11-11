import { menuViews } from './window'
import { View } from '../types/ui'

jest.mock('@tauri-apps/api/event', () => ({ listen: jest.fn() }))
jest.mock('@tauri-apps/api/core', () => ({ invoke: jest.fn() }))
jest.mock('@tauri-apps/plugin-dialog', () => ({ ask: jest.fn(), open: jest.fn(), save: jest.fn() }))
jest.mock('@tauri-apps/api/path', () => ({ dirname: jest.fn(), documentDir: jest.fn(), join: jest.fn() }))
jest.mock('@tauri-apps/plugin-fs', () => ({ readDir: jest.fn() }))

describe('menuViews', () => {
  it('opens Preferences from both of its menu items', () => {
    expect(menuViews.preferences).toBe(View.Preferences)
    expect(menuViews.appearance).toBe(View.Preferences)
  })
})
