import { useState } from 'react'
import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'

import type { AiSuggestion } from '../bindings/AiSuggestion'
import {
  draftsFromSuggestions,
  type AiSuggestionDraft,
} from '../lib/ai'
import { AiSuggestionReview } from './AiSuggestionReview'

const suggestion: AiSuggestion = {
  type: 'task',
  title: 'Buy milk',
  content: null,
  category: 'errands',
  tags: ['home'],
  priority: 'normal',
  due_at: null,
  remind_at: null,
}

function ReviewHarness({
  initial,
  onChange,
}: {
  initial: AiSuggestionDraft[]
  onChange?: (drafts: AiSuggestionDraft[]) => void
}) {
  const [drafts, setDrafts] = useState(initial)

  return (
    <MantineProvider>
      <AiSuggestionReview
        drafts={drafts}
        onChange={(next) => {
          setDrafts(next)
          onChange?.(next)
        }}
      />
    </MantineProvider>
  )
}

describe('AiSuggestionReview', () => {
  it('lets the user edit a suggestion before saving', async () => {
    const user = userEvent.setup()
    let latest: AiSuggestionDraft[] = []
    render(
      <ReviewHarness
        initial={draftsFromSuggestions([suggestion])}
        onChange={(drafts) => {
          latest = drafts
        }}
      />,
    )

    const title = screen.getByRole('textbox', {
      name: 'Suggestion 1 title',
    })
    await user.clear(title)
    await user.type(title, 'Buy oat milk')

    expect(latest[0].title).toBe('Buy oat milk')
  })

  it('can exclude and remove suggestions', async () => {
    const user = userEvent.setup()
    let latest: AiSuggestionDraft[] = []
    render(
      <ReviewHarness
        initial={draftsFromSuggestions([
          suggestion,
          { ...suggestion, title: 'Renew domain' },
        ])}
        onChange={(drafts) => {
          latest = drafts
        }}
      />,
    )

    await user.click(
      screen.getByRole('checkbox', { name: 'Include suggestion 2' }),
    )
    expect(latest[1].selected).toBe(false)

    await user.click(
      screen.getByRole('button', { name: 'Remove suggestion 1' }),
    )
    expect(latest).toHaveLength(1)
    expect(latest[0].title).toBe('Renew domain')
    expect(
      screen.queryByRole('textbox', { name: 'Suggestion 2 title' }),
    ).not.toBeInTheDocument()
  })
})
