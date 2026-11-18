import { useQuery } from '@tanstack/react-query'
import { get } from './client'

export interface CurrentUser {
  pid: string
  name: string
  email: string
}

export function useCurrentUser() {
  return useQuery({
    queryKey: ['current-user'],
    queryFn: () => get<CurrentUser>('/api/auth/current'),
    retry: false,
    staleTime: 5 * 60 * 1000,
  })
}
