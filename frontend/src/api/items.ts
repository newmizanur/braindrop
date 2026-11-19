import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import type { CreateItemsResponse } from '../bindings/CreateItemsResponse'
import type { CreateItemParams } from '../bindings/CreateItemParams'
import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemResponse } from '../bindings/ItemResponse'
import type { ItemSort } from '../bindings/ItemSort'
import type { ItemStatus } from '../bindings/ItemStatus'
import type { ItemType } from '../bindings/ItemType'
import type { UpdateItemParams } from '../bindings/UpdateItemParams'
import type { Page } from '../bindings/Page'
import type { SortOrder } from '../bindings/SortOrder'
import { del, get, patch, post } from './client'

export const ITEMS_QUERY_KEY = ['items'] as const
export const STATS_QUERY_KEY = ['stats'] as const

export interface ItemFilters {
  q?: string
  type?: ItemType
  statuses?: ItemStatus[]
  priority?: ItemPriority
  category?: string
  tag?: string
  dueFrom?: string
  dueTo?: string
  sort?: ItemSort
  order?: SortOrder
  page?: number
  pageSize?: number
}

/** The generated request DTOs are the API contract for item writes. */
export type ItemInput = CreateItemParams
export type ItemUpdate = UpdateItemParams

/**
 * Turn the UI's camelCase filter object into the API's query string. Empty
 * values are dropped so a blank search box does not send `q=`.
 */
export function buildItemsQuery(filters: ItemFilters): string {
  const params = new URLSearchParams()
  const set = (key: string, value: string | number | undefined) => {
    if (value === undefined || value === '') {
      return
    }
    params.set(key, String(value))
  }

  set('q', filters.q?.trim())
  set('type', filters.type)
  set('status', filters.statuses?.join(','))
  set('priority', filters.priority)
  set('category', filters.category?.trim())
  set('tag', filters.tag?.trim())
  set('due_from', filters.dueFrom)
  set('due_to', filters.dueTo)
  set('sort', filters.sort)
  set('order', filters.order)
  set('page', filters.page)
  set('page_size', filters.pageSize)

  const query = params.toString()
  return query ? `?${query}` : ''
}

export function useItems(filters: ItemFilters = {}) {
  return useQuery({
    queryKey: [...ITEMS_QUERY_KEY, filters],
    queryFn: () => get<Page<ItemResponse>>(`/api/items${buildItemsQuery(filters)}`),
    placeholderData: (previous) => previous,
  })
}

export function useCreateItems() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (items: ItemInput[]) =>
      post<CreateItemsResponse>('/api/items', { items }),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ITEMS_QUERY_KEY })
      void queryClient.invalidateQueries({ queryKey: STATS_QUERY_KEY })
    },
  })
}

export function useUpdateItem() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: ({ pid, update }: { pid: string; update: ItemUpdate }) =>
      patch<ItemResponse>(`/api/items/${pid}`, update),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ITEMS_QUERY_KEY })
      void queryClient.invalidateQueries({ queryKey: STATS_QUERY_KEY })
    },
  })
}

export function useDeleteItem() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (pid: string) => del(`/api/items/${pid}`),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ITEMS_QUERY_KEY })
      void queryClient.invalidateQueries({ queryKey: STATS_QUERY_KEY })
    },
  })
}
