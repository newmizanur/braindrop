import { useQuery } from '@tanstack/react-query'

import type { Stats } from '../bindings/Stats'
import { get } from './client'
import { STATS_QUERY_KEY } from './items'

export function useStats() {
  return useQuery({
    queryKey: STATS_QUERY_KEY,
    queryFn: () => get<Stats>('/api/stats'),
  })
}
