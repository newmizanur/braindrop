import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import type { ItemResponse } from '../bindings/ItemResponse'
import { ItemCard } from './ItemCard'

const item: ItemResponse = {
  pid: '11111111-1111-1111-1111-111111111111',
  type: 'task',
  title: 'Buy milk',
  content: 'Semi-skimmed',
  category: 'errands',
  tags: ['home'],
  status: 'inbox',
  priority: 'high',
  due_at: null,
  remind_at: null,
  completed_at: null,
  archived_at: null,
  created_at: '2026-11-18T10:00:00Z',
  updated_at: '2026-11-18T10:00:00Z',
}

function renderCard(overrides: Partial<ItemResponse> = {}) {
  const handlers = {
    onToggle: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
  }
  render(
    <MantineProvider env="test">
      <ItemCard item={{ ...item, ...overrides }} {...handlers} />
    </MantineProvider>,
  )
  return handlers
}

describe('ItemCard', () => {
  it('renders the item title, type, priority and tags', () => {
    renderCard()

    expect(screen.getByText('Buy milk')).toBeInTheDocument()
    expect(screen.getByText('Task')).toBeInTheDocument()
    expect(screen.getByText('High')).toBeInTheDocument()
    expect(screen.getByText('#home')).toBeInTheDocument()
    expect(screen.getByText('Semi-skimmed')).toBeInTheDocument()
  })

  it('calls onToggle when the completion checkbox is used', async () => {
    const user = userEvent.setup()
    const handlers = renderCard()

    await user.click(screen.getByRole('checkbox', { name: 'Complete Buy milk' }))

    expect(handlers.onToggle).toHaveBeenCalledTimes(1)
    expect(handlers.onToggle).toHaveBeenCalledWith(
      expect.objectContaining({ pid: item.pid }),
    )
  })

  it('shows a reopen action for completed items', () => {
    renderCard({ status: 'completed', completed_at: '2026-11-18T11:00:00Z' })

    expect(
      screen.getByRole('checkbox', { name: 'Reopen Buy milk' }),
    ).toBeChecked()
  })

  it('exposes edit and delete in the actions menu', async () => {
    const user = userEvent.setup()
    const handlers = renderCard()

    await user.click(
      screen.getByRole('button', { name: 'Actions for Buy milk' }),
    )
    await user.click(await screen.findByRole('menuitem', { name: 'Edit' }))

    expect(handlers.onEdit).toHaveBeenCalledWith(
      expect.objectContaining({ pid: item.pid }),
    )
  })
})
