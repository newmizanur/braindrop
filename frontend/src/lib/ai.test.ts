import { describe, expect, it } from 'vitest'

import type { AiSuggestion } from '../bindings/AiSuggestion'
import {
  aiErrorMessage,
  draftsFromSuggestions,
  selectedItemInputs,
} from './ai'

const suggestion: AiSuggestion = {
  type: 'task',
  title: 'Buy milk',
  content: 'Semi-skimmed',
  category: 'errands',
  tags: ['home', 'shopping'],
  priority: 'high',
  due_at: '2026-12-15T09:00:00Z',
  remind_at: null,
}

describe('draftsFromSuggestions', () => {
  it('selects every suggestion and turns nullable fields into editable text', () => {
    const [draft] = draftsFromSuggestions([suggestion])

    expect(draft).toMatchObject({
      selected: true,
      type: 'task',
      title: 'Buy milk',
      content: 'Semi-skimmed',
      category: 'errands',
      tags: 'home, shopping',
      priority: 'high',
      dueAt: '2026-12-15T09:00:00Z',
      remindAt: null,
    })
  })
})

describe('selectedItemInputs', () => {
  it('only confirms selected suggestions with a title', () => {
    const drafts = draftsFromSuggestions([
      suggestion,
      { ...suggestion, title: 'Not selected', due_at: null },
      { ...suggestion, title: '   ' },
    ])
    drafts[1].selected = false

    const items = selectedItemInputs(drafts)

    expect(items).toHaveLength(1)
    expect(items[0]).toEqual({
      type: 'task',
      title: 'Buy milk',
      content: 'Semi-skimmed',
      category: 'errands',
      tags: ['home', 'shopping'],
      priority: 'high',
      due_at: '2026-12-15T09:00:00.000Z',
      remind_at: null,
    })
  })

  it('applies edits and turns cleared optional fields into nulls', () => {
    const [draft] = draftsFromSuggestions([suggestion])
    draft.title = '  Buy oat milk  '
    draft.content = '   '
    draft.category = ''
    draft.tags = 'home, home, errands'
    draft.dueAt = null

    expect(selectedItemInputs([draft])[0]).toMatchObject({
      title: 'Buy oat milk',
      content: null,
      category: null,
      tags: ['home', 'errands'],
      due_at: null,
    })
  })
})

describe('aiErrorMessage', () => {
  it('maps known API codes to actionable messages', () => {
    expect(aiErrorMessage({ body: { code: 'ai_disabled' } })).toContain(
      'still save manually',
    )
    expect(
      aiErrorMessage({ body: { code: 'ai_authentication' } }),
    ).toContain('Integrations')
    expect(
      aiErrorMessage({
        body: { code: 'unknown_code', message: 'Provider said no' },
      }),
    ).toBe('Provider said no')
  })

  it('falls back to the error message and then a generic message', () => {
    expect(aiErrorMessage(new Error('network down'))).toBe('network down')
    expect(aiErrorMessage(undefined)).toContain('Try again')
  })
})
