import type { ItemInput } from '../api/items'
import type { AiSuggestion } from '../bindings/AiSuggestion'
import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemType } from '../bindings/ItemType'
import { parseTags, toIso } from './items'

/**
 * The editable form of one AI suggestion. The API returns ISO instants; the
 * review UI needs plain strings so the user can clear or rewrite any field.
 */
export interface AiSuggestionDraft {
  key: string
  selected: boolean
  type: ItemType
  title: string
  content: string
  category: string
  tags: string
  priority: ItemPriority
  dueAt: string | Date | null
  remindAt: string | Date | null
}

export function draftsFromSuggestions(
  suggestions: AiSuggestion[],
): AiSuggestionDraft[] {
  return suggestions.map((suggestion, index) => ({
    key: `ai-${index}`,
    selected: true,
    type: suggestion.type,
    title: suggestion.title,
    content: suggestion.content ?? '',
    category: suggestion.category ?? '',
    tags: suggestion.tags.join(', '),
    priority: suggestion.priority,
    dueAt: suggestion.due_at,
    remindAt: suggestion.remind_at,
  }))
}

/** Convert one reviewed draft to the item creation payload, or `null` when it
 * has no usable title. Blank optional fields become explicit nulls. */
export function draftToItemInput(draft: AiSuggestionDraft): ItemInput | null {
  const title = draft.title.trim()
  if (!title) {
    return null
  }

  return {
    type: draft.type,
    title,
    content: draft.content.trim() || null,
    category: draft.category.trim() || null,
    tags: parseTags(draft.tags),
    priority: draft.priority,
    due_at: toIso(draft.dueAt),
    remind_at: toIso(draft.remindAt),
  }
}

/** The payload for `POST /api/items`: only selected drafts with a title. */
export function selectedItemInputs(drafts: AiSuggestionDraft[]): ItemInput[] {
  return drafts
    .filter((draft) => draft.selected)
    .map(draftToItemInput)
    .filter((item): item is ItemInput => item !== null)
}

const errorMessages: Record<string, string> = {
  ai_disabled:
    'AI is disabled for this account or server. You can still save manually.',
  ai_not_configured:
    'AI is not configured yet. Add a key in Integrations, or save manually.',
  ai_rate_limited:
    'AI is receiving too many requests. Wait a moment and try again.',
  ai_timeout:
    'The AI provider took too long to respond. Try again or save manually.',
  ai_authentication:
    'The AI provider rejected the API key. Check the key in Integrations.',
  ai_provider_error:
    'The AI provider is unavailable right now. Try again or save manually.',
  ai_invalid_response:
    'The AI provider returned an unexpected response. Try again or save manually.',
  ai_unsupported_response_format:
    'This model does not support structured output. Choose another model in Integrations.',
  invalid_input:
    'The text could not be processed. Check the length and try again.',
}

/** Turn an API or network failure into an actionable, secret-free message. */
export function aiErrorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null) {
    const body = (error as { body?: { code?: unknown; message?: unknown } }).body
    if (body && typeof body.code === 'string') {
      return errorMessages[body.code] ?? String(body.message ?? body.code)
    }
  }
  if (error instanceof Error && error.message) {
    return error.message
  }
  return 'Could not organize this capture. Try again or save it manually.'
}
