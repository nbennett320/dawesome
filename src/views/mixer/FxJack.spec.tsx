import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import FxJack from './FxJack'

const socket = (container: HTMLElement) => container.querySelector('.ft-jack')
const led = (container: HTMLElement) => container.querySelector('.ft-led')?.getAttribute('data-state')

describe('the FX jack', () => {
  it('is an empty socket, labelled FX, with no effects', () => {
    const onOpen = vi.fn()
    const { container } = render(<FxJack channel='Track 1' count={0} lit={false} onOpen={onOpen} />)

    const jack = screen.getByRole('button', { name: 'Track 1 effects' })
    expect(jack.textContent).toBe('FX')
    expect(socket(container)?.getAttribute('data-plugged')).toBe('false')
    expect(container.querySelector('.ft-jack__body')).toBeNull()
    expect(led(container)).toBe('off')
    expect(jack.getAttribute('title')).toMatch(/^Open the Effects Rack, or drag an effect .* onto the strip/)

    fireEvent.click(jack)
    expect(onOpen).toHaveBeenCalledTimes(1)
  })

  it('has a plug in it and the count beside it with effects, lit while one is on', () => {
    const { container, rerender } = render(<FxJack channel='Track 1' count={3} lit onOpen={() => {}} />)

    const jack = screen.getByRole('button', { name: 'Track 1 effects' })
    expect(socket(container)?.getAttribute('data-plugged')).toBe('true')
    expect(container.querySelector('.ft-jack__cable')).toBeTruthy()
    expect(screen.getByTestId('fx-count').textContent).toBe('3')
    expect(led(container)).toBe('on')
    expect(jack.getAttribute('title')).toMatch(/^3 effects\. Open them in the Effects Rack, or drag/)

    // all bypassed: still plugged, but dark
    rerender(<FxJack channel='Track 1' count={3} lit={false} onOpen={() => {}} />)
    expect(socket(container)?.getAttribute('data-plugged')).toBe('true')
    expect(led(container)).toBe('off')
    expect(jack.getAttribute('title')).toMatch(/^3 effects, all off\./)
  })

  it('marks a track with an instrument, counted with its effects', () => {
    const { container, rerender } = render(<FxJack channel='Track 1' count={2} lit instrument onOpen={() => {}} />)
    const count = screen.getByTestId('fx-count')
    expect(count.textContent).toBe('2')
    expect(count.querySelector('svg')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Track 1 effects' }).getAttribute('title')).toMatch(/^An instrument and 1 effect\./)

    rerender(<FxJack channel='Track 1' count={2} lit onOpen={() => {}} />)
    expect(container.querySelector('[data-testid="fx-count"] svg')).toBeNull()
  })
})
