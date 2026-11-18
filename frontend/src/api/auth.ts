import { useQuery } from '@tanstack/react-query'
import { get } from './client'

export interface CurrentUser {
  pid: string
  name: string
  email: string
}

interface UseCurrentUserOptions {
  enabled?: boolean
}

export function useCurrentUser({ enabled = true }: UseCurrentUserOptions = {}) {
  return useQuery({
    queryKey: ['current-user'],
    queryFn: () => get<CurrentUser>('/api/auth/current'),
    enabled,
    retry: false,
    staleTime: 5 * 60 * 1000,
  })
}
