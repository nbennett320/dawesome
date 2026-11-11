import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import FxButton from './FxButton'

describe('FxButton', () => {
  it('shows the count, and lights while an effect is on', () => {
    const onOpen = vi.fn()
    const { container, rerender } = render(<FxButton channel='Track 1' count={3} lit onOpen={onOpen} />)

    const button = screen.getByRole('button', { name: 'Track 1 effects' })
    expect(button.textContent).toContain('FX 3')
    expect(container.querySelector('.ft-led')?.getAttribute('data-state')).toBe('on')

    fireEvent.click(button)
    expect(onOpen).toHaveBeenCalledTimes(1)

    rerender(<FxButton channel='Track 1' count={0} lit={false} onOpen={onOpen} />)
    expect(button.textContent).toMatch(/^FX$/)
    expect(container.querySelector('.ft-led')?.getAttribute('data-state')).toBe('off')
  })

  it('marks a track with an instrument, counted with its effects', () => {
    const { container, rerender } = render(<FxButton channel='Track 1' count={2} lit instrument onOpen={() => {}} />)
    const button = screen.getByRole('button', { name: 'Track 1 effects' })
    expect(button.textContent).toBe('FX 2')
    expect(button.querySelector('svg')).toBeTruthy()
    expect(button.getAttribute('title')).toMatch(/^An instrument and 1 effect\./)

    rerender(<FxButton channel='Track 1' count={2} lit onOpen={() => {}} />)
    expect(container.querySelector('button svg')).toBeNull()
  })
})
