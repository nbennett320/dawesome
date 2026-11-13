import React from 'react'
import { describe, expect, it, vi, afterEach } from 'vitest'
import { act, fireEvent, render, screen } from '@testing-library/react'
import RowControls from './RowControls'

const setup = (withRename = true) => {
  const onName = vi.fn()
  const onRename = vi.fn()
  const onPan = vi.fn()
  render(
    <RowControls
      name='Snare'
      note='assets_66-sd-01'
      nameTitle='Click to hear it'
      onName={onName}
      rename={withRename ? { initial: 'Snare', placeholder: 'assets_66-sd-01', onRename } : undefined}
      muted={false}
      onMute={() => {}}
      volume={1}
      onVolume={() => {}}
      pan={-0.4}
      onPan={onPan}
      removeTitle='Remove'
      onRemove={() => {}}
    />,
  )
  return { onName, onRename, onPan, name: screen.getByTitle('Click to hear it') }
}

afterEach(() => { vi.useRealTimers() })

describe("a sequencer row's controls", () => {
  it('shows the name, then the sample faintly', () => {
    const { name } = setup()
    expect(name.textContent).toBe('Snareassets_66-sd-01')
  })

  it('does what a click does once it is sure it is not a double-click', () => {
    vi.useFakeTimers()
    const { onName, name } = setup()
    fireEvent.click(name, { detail: 1 })
    expect(onName).not.toHaveBeenCalled()
    act(() => { vi.advanceTimersByTime(300) })
    expect(onName).toHaveBeenCalledOnce()
  })

  it('renames on a double-click, without doing what a click does', () => {
    vi.useFakeTimers()
    const { onName, onRename, name } = setup()
    fireEvent.click(name, { detail: 1 })
    fireEvent.click(name, { detail: 2 })
    fireEvent.doubleClick(name)
    act(() => { vi.advanceTimersByTime(300) })
    expect(onName).not.toHaveBeenCalled()

    const input = screen.getByLabelText('Row name') as HTMLInputElement
    expect(input.value).toBe('Snare')
    fireEvent.change(input, { target: { value: 'Rim' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(onRename).toHaveBeenCalledExactlyOnceWith('Rim')
    expect(screen.queryByLabelText('Row name')).toBeNull()
  })

  it('leaves the name as it was on Escape', () => {
    const { onRename, name } = setup()
    fireEvent.doubleClick(name)
    fireEvent.keyDown(screen.getByLabelText('Row name'), { key: 'Escape' })
    expect(onRename).not.toHaveBeenCalled()
  })

  it('does what a click does straight away when it cannot be renamed', () => {
    const { onName, name } = setup(false)
    fireEvent.click(name, { detail: 1 })
    expect(onName).toHaveBeenCalledOnce()
    fireEvent.doubleClick(name)
    expect(screen.queryByLabelText('Row name')).toBeNull()
  })

  it('has a pan knob', () => {
    setup()
    const knob = screen.getByRole('slider', { name: 'Snare pan' })
    expect(knob.getAttribute('aria-valuetext')).toBe('L 40')
  })
})
