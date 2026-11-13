import React from 'react'
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import RowControls from './RowControls'

const setup = () => {
  const onName = vi.fn()
  const onRename = vi.fn()
  const onGated = vi.fn()
  const onPan = vi.fn()
  const onRemove = vi.fn()
  render(
    <RowControls
      name='Snare'
      note='assets_66-sd-01'
      nameTitle='Click to hear it'
      onName={onName}
      menu={[
        {
          label: 'Rename…',
          field: { label: 'Row name', initial: 'Snare', placeholder: 'assets_66-sd-01', maxLength: 40, onDone: onRename },
        },
        { label: 'Gated', onSelect: onGated },
      ]}
      muted={false}
      onMute={() => {}}
      volume={1}
      onVolume={() => {}}
      pan={-0.4}
      onPan={onPan}
      onGripDown={() => {}}
      removeTitle='Remove'
      onRemove={onRemove}
    />,
  )
  return { onName, onRename, onGated, onPan, onRemove, name: screen.getByTitle('Click to hear it') }
}

describe("a Pattern Sequencer row's controls", () => {
  it('shows the name, then the sample faintly', () => {
    const { name } = setup()
    expect(name.textContent).toBe('Snareassets_66-sd-01')
  })

  it('does what a click does straight away', () => {
    const { onName, name } = setup()
    fireEvent.click(name)
    expect(onName).toHaveBeenCalledOnce()
  })

  it('opens a field in place of the name from the right-click menu', () => {
    const { onName, onRename, name } = setup()
    fireEvent.contextMenu(name)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Rename…' }))
    expect(screen.queryByRole('menu')).toBeNull()
    expect(onName).not.toHaveBeenCalled()

    const input = screen.getByLabelText('Row name') as HTMLInputElement
    expect(input.value).toBe('Snare')
    expect(input.placeholder).toBe('assets_66-sd-01')
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

  it('does what the other items of the menu say, then Remove', () => {
    const { onGated, onRemove, name } = setup()
    fireEvent.contextMenu(name)
    expect(screen.getAllByRole('menuitem').map(item => item.textContent)).toEqual(['Rename…', 'Gated', 'Remove'])
    fireEvent.click(screen.getByRole('menuitem', { name: 'Gated' }))
    expect(onGated).toHaveBeenCalledOnce()
    fireEvent.contextMenu(name)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Remove' }))
    expect(onRemove).toHaveBeenCalledOnce()
  })

  it('has a pan knob', () => {
    setup()
    const knob = screen.getByRole('slider', { name: 'Snare pan' })
    expect(knob.getAttribute('aria-valuetext')).toBe('L 40')
  })
})
