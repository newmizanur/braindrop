import { useMutation } from '@tanstack/react-query'

import type { AiParseResponse } from '../bindings/AiParseResponse'
import type { ParseItemsParams } from '../bindings/ParseItemsParams'
import { post } from './client'

/**
 * Ask the server to turn unstructured capture text into reviewable item
 * suggestions. Nothing is persisted by this call; the user saves the
 * suggestions they keep through `POST /api/items`.
 */
export function useParseItems() {
  return useMutation({
    mutationFn: (text: string) =>
      post<AiParseResponse>('/api/ai/parse', {
        text,
      } satisfies ParseItemsParams),
  })
}
