import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import TrackColorPicker from './TrackColorPicker'
import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'

beforeAll(() => applyTheme(slateLightTheme))

describe('the track color jewel', () => {
  it('is an unlit LED in the track’s color, handed to whoever lights it', () => {
    let led: HTMLElement | null = null
    render(<TrackColorPicker label='Drums' color={2} onChange={vi.fn()} ledRef={l => { led = l }} />)

    const jewel = screen.getByRole('button', { name: 'Drums color' })
    expect(led).not.toBeNull()
    expect(jewel.contains(led)).toBe(true)
    expect(led!.getAttribute('data-state')).toBe('off')
    expect(led!.style.getPropertyValue('--led')).not.toBe('')
  })

  it('opens a tray of lit LEDs, the current one checked, and picks from it', () => {
    const onChange = vi.fn()
    render(<TrackColorPicker label='Drums' color={2} onChange={onChange} />)

    fireEvent.click(screen.getByRole('button', { name: 'Drums color' }))
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

  it('closes on Escape, or a press outside', () => {
    render(<TrackColorPicker label='Drums' color={0} onChange={vi.fn()} />)
    const jewel = screen.getByRole('button', { name: 'Drums color' })

    fireEvent.click(jewel)
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu')).toBeNull()

    fireEvent.click(jewel)
    fireEvent.pointerDown(document.body)
    expect(screen.queryByRole('menu')).toBeNull()
  })
})
