import React from 'react'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import ChannelJewel from './ChannelJewel'
import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'

beforeAll(() => applyTheme(slateLightTheme))

const jewel = (props: Partial<React.ComponentProps<typeof ChannelJewel>> = {}) => {
  const handlers = { onChange: vi.fn(), onMute: vi.fn(), onSolo: vi.fn() }
  render(<ChannelJewel label='Drums' color={2} muted={false} soloed={false} {...handlers} {...props} />)
  return { ...handlers, button: screen.getByRole('button', { name: /^Drums/ }) }
}

// let a menu's close (which waits a microtask) happen
const settle = () => act(async () => { await Promise.resolve() })

describe('the channel jewel', () => {
  it('is an unlit LED in the track’s color, handed to whoever lights it', () => {
    let led: HTMLElement | null = null
    const { button } = jewel({ ledRef: (l: HTMLElement | null) => { led = l } })

    expect(led).not.toBeNull()
    expect(button.contains(led)).toBe(true)
    expect(led!.getAttribute('data-state')).toBe('off')
    expect(led!.style.getPropertyValue('--led')).not.toBe('')
  })

  it('mutes on a click, and solos on a shift+click', () => {
    const { button, onMute, onSolo } = jewel({ muted: true })

    fireEvent.click(button)
    expect(onMute).toHaveBeenCalledWith(false)
    expect(onSolo).not.toHaveBeenCalled()

    fireEvent.click(button, { shiftKey: true })
    expect(onSolo).toHaveBeenCalledWith(true)
    expect(onMute).toHaveBeenCalledTimes(1)
  })

  it('opens its menu on a right-click, which opens the tray of colors', async () => {
    const { button, onChange, onMute } = jewel()

    fireEvent.contextMenu(button, { clientX: 40, clientY: 50 })
    expect(onMute).not.toHaveBeenCalled()
    const items = screen.getAllByRole('menuitem')
    expect(items.map(i => i.textContent)).toEqual(['Change channel color'])

    fireEvent.click(items[0])
    await settle()
    const tray = screen.getByRole('menu', { name: 'Drums color' })
    const colors = screen.getAllByRole('menuitemradio')
    expect(colors).toHaveLength(8)
    expect(colors.map(c => c.getAttribute('aria-checked'))).toEqual(
      ['false', 'false', 'true', 'false', 'false', 'false', 'false', 'false'],
    )
    expect([...tray.querySelectorAll('.ft-led')].every(l => l.getAttribute('data-state') === 'on')).toBe(true)

    fireEvent.click(screen.getByRole('menuitemradio', { name: 'blue' }))
    expect(onChange).toHaveBeenCalledWith(5)
    expect(screen.queryByRole('menu')).toBeNull()
  })

  it('arms or disarms its track from its menu, when it can be armed', async () => {
    const onToggle = vi.fn()
    const { button } = jewel({ arm: { armed: false, onToggle } })

    fireEvent.contextMenu(button)
    expect(screen.getAllByRole('menuitem').map(i => i.textContent)).toEqual(
      ['Arm for recording', 'Change channel color'],
    )
    fireEvent.click(screen.getByRole('menuitem', { name: 'Arm for recording' }))
    await settle()
    expect(onToggle).toHaveBeenCalledTimes(1)
    expect(screen.queryByRole('menu')).toBeNull()

    cleanup()
    const armed = jewel({ arm: { armed: true, onToggle } })
    fireEvent.contextMenu(armed.button)
    expect(screen.getByRole('menuitem', { name: 'Disarm recording' })).toBeTruthy()
  })

  it('closes its menu on Escape, and the tray on a press outside', async () => {
    const { button } = jewel()

    fireEvent.contextMenu(button)
    fireEvent.keyDown(window, { key: 'Escape' })
    await settle()
    expect(screen.queryByRole('menu')).toBeNull()

    fireEvent.contextMenu(button)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Change channel color' }))
    await settle()
    expect(screen.getByRole('menu', { name: 'Drums color' })).toBeTruthy()
    fireEvent.pointerDown(document.body)
    expect(screen.queryByRole('menu')).toBeNull()
  })
})
