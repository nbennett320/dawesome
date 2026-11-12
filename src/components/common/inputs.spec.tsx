import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { NameInput, ReadoutInput } from '.'

describe('a readout input', () => {
  const setup = () => {
    const onCommit = vi.fn()
    render(<ReadoutInput aria-label='level' value='-6' onCommit={onCommit} />)
    return { onCommit, input: screen.getByLabelText('level') as HTMLInputElement }
  }

  it('reads the value until something is typed', () => {
    const { input } = setup()
    expect(input.value).toBe('-6')
    fireEvent.change(input, { target: { value: '-3' } })
    expect(input.value).toBe('-3')
  })

  it('hands over what was typed when focus leaves', () => {
    const { input, onCommit } = setup()
    fireEvent.change(input, { target: { value: '-3' } })
    fireEvent.blur(input)
    expect(onCommit).toHaveBeenCalledExactlyOnceWith('-3')
    expect(input.value).toBe('-6')
  })

  it('commits on enter, by letting go of focus', () => {
    const { input, onCommit } = setup()
    input.focus()
    fireEvent.change(input, { target: { value: '-3' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(onCommit).toHaveBeenCalledExactlyOnceWith('-3')
  })

  it('drops what was typed on escape', () => {
    const { input, onCommit } = setup()
    fireEvent.change(input, { target: { value: '-3' } })
    fireEvent.keyDown(input, { key: 'Escape' })
    fireEvent.blur(input)
    expect(onCommit).not.toHaveBeenCalled()
    expect(input.value).toBe('-6')
  })

  it('has nothing to commit when nothing was typed', () => {
    const { input, onCommit } = setup()
    fireEvent.blur(input)
    expect(onCommit).not.toHaveBeenCalled()
  })
})

describe('a name input', () => {
  const setup = () => {
    const onDone = vi.fn()
    render(<NameInput aria-label='name' initial='Kick' onDone={onDone} />)
    return { onDone, input: screen.getByLabelText('name') as HTMLInputElement }
  }

  it('starts with the name it has', () => {
    expect(setup().input.value).toBe('Kick')
  })

  it('hands over the new name on enter', () => {
    const { input, onDone } = setup()
    fireEvent.change(input, { target: { value: 'Snare' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    expect(onDone).toHaveBeenCalledExactlyOnceWith('Snare')
  })

  it('hands it over once, though focus leaves after enter', () => {
    const { input, onDone } = setup()
    fireEvent.change(input, { target: { value: 'Snare' } })
    fireEvent.keyDown(input, { key: 'Enter' })
    fireEvent.blur(input)
    expect(onDone).toHaveBeenCalledTimes(1)
  })

  it('hands it over when focus leaves', () => {
    const { input, onDone } = setup()
    fireEvent.change(input, { target: { value: 'Hat' } })
    fireEvent.blur(input)
    expect(onDone).toHaveBeenCalledExactlyOnceWith('Hat')
  })

  it('hands over nothing on escape', () => {
    const { input, onDone } = setup()
    fireEvent.change(input, { target: { value: 'Snare' } })
    fireEvent.keyDown(input, { key: 'Escape' })
    fireEvent.blur(input)
    expect(onDone).toHaveBeenCalledExactlyOnceWith(null)
  })
})
