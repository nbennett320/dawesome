import React from 'react'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import ChannelStrip, { TapeControl } from './ChannelStrip'
import { applyTheme } from '../../theme/apply'
import { slateLightTheme } from '../../theme/fixture'

beforeAll(() => applyTheme(slateLightTheme))
afterEach(cleanup)

const strip = (tape?: Partial<TapeControl>) => {
  const handlers = { onTapeChange: vi.fn(), onDrive: vi.fn() }
  render(
    <ChannelStrip
      label='Drums'
      volumeDb={0}
      onVolume={vi.fn()}
      meterKey='track-0'
      register={vi.fn()}
      tape={tape && { onTape: false, driveDb: 0, tapeOn: true, ...handlers, ...tape }}
    />,
  )
  return handlers
}

const tapeKey = () => screen.getByRole('button', { name: 'Put Drums on tape' })
const drive = () => screen.getByRole('slider', { name: 'Drums tape drive' })
// the LED beside the key
const led = () => tapeKey().parentElement!.querySelector('.ft-led')!

describe('a strip’s tape controls', () => {
  it('put the track on tape, with its light lit', () => {
    const { onTapeChange } = strip({ onTape: false })
    expect(tapeKey().getAttribute('aria-pressed')).toBe('false')
    expect(led().getAttribute('data-state')).toBe('off')
    fireEvent.click(tapeKey())
    expect(onTapeChange).toHaveBeenCalledWith(true)
    cleanup()

    strip({ onTape: true })
    expect(tapeKey().getAttribute('aria-pressed')).toBe('true')
    expect(led().getAttribute('data-state')).toBe('on')
  })

  it('dim the drive off tape, or with the tape off, but it still turns', () => {
    const { onDrive } = strip({ onTape: false })
    expect(drive().parentElement!.className).toContain('opacity-50')
    fireEvent.keyDown(drive(), { key: 'ArrowUp' })
    expect(onDrive).toHaveBeenCalledWith(0.5)
    cleanup()

    strip({ onTape: true, tapeOn: false })
    expect(drive().parentElement!.className).toContain('opacity-50')
    // on tape, with the tape off: a dim light
    expect(led().getAttribute('data-state')).toBe('dim')
    cleanup()

    strip({ onTape: true, tapeOn: true })
    expect(drive().parentElement!.className).not.toContain('opacity-50')
  })

  it('go back to 0 dB on a double-click, and stay in range', () => {
    const { onDrive } = strip({ onTape: true, driveDb: 6 })
    fireEvent.doubleClick(drive())
    expect(onDrive).toHaveBeenLastCalledWith(0)
    fireEvent.keyDown(drive(), { key: 'End' })
    expect(onDrive).toHaveBeenLastCalledWith(18)
    fireEvent.keyDown(drive(), { key: 'Home' })
    expect(onDrive).toHaveBeenLastCalledWith(-12)
  })

  it('aren’t on the master', () => {
    strip()
    expect(screen.queryByRole('button', { name: /on tape/ })).toBeNull()
    expect(screen.queryByRole('slider', { name: /drive/ })).toBeNull()
  })
})
