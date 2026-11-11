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
})
