import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import type { SettingsResponse } from '../bindings/SettingsResponse'
import { get, patch } from './client'
import { ITEMS_QUERY_KEY, STATS_QUERY_KEY } from './items'

export const SETTINGS_QUERY_KEY = ['settings'] as const

export type SettingsUpdate = Partial<SettingsResponse>

export function useSettings() {
  return useQuery({
    queryKey: SETTINGS_QUERY_KEY,
    queryFn: () => get<SettingsResponse>('/api/settings'),
    staleTime: 5 * 60 * 1000,
  })
}

export function useUpdateSettings() {
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (update: SettingsUpdate) =>
      patch<SettingsResponse>('/api/settings', update),
    onSuccess: (settings) => {
      queryClient.setQueryData(SETTINGS_QUERY_KEY, settings)
      // Timezone changes move the day boundaries used by stats and date
      // filters, so both caches must be refreshed.
      void queryClient.invalidateQueries({ queryKey: STATS_QUERY_KEY })
      void queryClient.invalidateQueries({ queryKey: ITEMS_QUERY_KEY })
    },
  })
}
