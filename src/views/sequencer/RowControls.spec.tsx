import React from 'react'
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import RowControls from './RowControls'

const setup = (withRename = true) => {
  const onName = vi.fn()
  const onRename = vi.fn()
  const onPan = vi.fn()
  const onRemove = vi.fn()
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
      onRemove={onRemove}
    />,
  )
  return { onName, onRename, onPan, onRemove, name: screen.getByTitle('Click to hear it') }
}

describe("a sequencer row's controls", () => {
  it('shows the name, then the sample faintly', () => {
    const { name } = setup()
    expect(name.textContent).toBe('Snareassets_66-sd-01')
  })

  it('does what a click does straight away', () => {
    const { onName, name } = setup()
    fireEvent.click(name)
    expect(onName).toHaveBeenCalledOnce()
  })

  it('renames from the right-click menu', () => {
    const { onName, onRename, name } = setup()
    fireEvent.contextMenu(name)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Rename…' }))
    expect(screen.queryByRole('menu')).toBeNull()
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
    fireEvent.contextMenu(name)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Rename…' }))
    fireEvent.keyDown(screen.getByLabelText('Row name'), { key: 'Escape' })
    expect(onRename).not.toHaveBeenCalled()
  })

  it('removes from the right-click menu', () => {
    const { onRemove, name } = setup()
    fireEvent.contextMenu(name)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Remove' }))
    expect(onRemove).toHaveBeenCalledOnce()
  })

  it('has no Rename when it cannot be renamed', () => {
    const { name } = setup(false)
    fireEvent.contextMenu(name)
    expect(screen.queryByRole('menuitem', { name: 'Rename…' })).toBeNull()
    expect(screen.getByRole('menuitem', { name: 'Remove' })).toBeTruthy()
  })

  it('has a pan knob', () => {
    setup()
    const knob = screen.getByRole('slider', { name: 'Snare pan' })
    expect(knob.getAttribute('aria-valuetext')).toBe('L 40')
  })
})
