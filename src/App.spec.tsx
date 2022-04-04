import React from 'react'
import { render, screen } from '@testing-library/react'
import App from './App'

test('renders learn react link', () => {
  render(<App />)
  const playButton = screen.getByText(/Play/i)
  expect(playButton).toBeInTheDocument()
})
