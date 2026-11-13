import React from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { SidebarPage } from '../../types/ui'
import SidebarList from './SidebarList'

describe('the sidebar rail', () => {
  it('has one key for the project\'s patterns, among a key for each page', () => {
    const onPress = vi.fn()
    render(<SidebarList open page={SidebarPage.Samples} onPress={onPress} />)
    expect(screen.getAllByRole('button')).toHaveLength(Object.values(SidebarPage).length)
    expect(screen.getAllByRole('button', { name: /patterns/i }).map(key => key.getAttribute('aria-label')))
      .toEqual(['Patterns'])

    fireEvent.click(screen.getByRole('button', { name: 'Patterns' }))
    expect(onPress).toHaveBeenCalledWith(SidebarPage.Patterns)
  })
})
