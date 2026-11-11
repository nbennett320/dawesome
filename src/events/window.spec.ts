import { menuViews } from './window'
import { View } from '../types/ui'

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn(), open: vi.fn(), save: vi.fn() }))
vi.mock('@tauri-apps/api/path', () => ({ dirname: vi.fn(), documentDir: vi.fn(), join: vi.fn() }))
vi.mock('@tauri-apps/plugin-fs', () => ({ readDir: vi.fn() }))

describe('menuViews', () => {
  it('opens Preferences from both of its menu items', () => {
    expect(menuViews.preferences).toBe(View.Preferences)
    expect(menuViews.appearance).toBe(View.Preferences)
  })
})
