import { useQuery } from '@tanstack/react-query'
import { get } from './client'

export interface PublicAuthConfig {
  allow_registration: boolean
  require_email_verification: boolean
}

/**
 * Public authentication policy. The endpoint is intentionally unauthenticated
 * so the login and registration screens can render correctly before a token
 * exists. It carries no user data.
 */
export function usePublicAuthConfig() {
  return useQuery({
    queryKey: ['public-auth-config'],
    queryFn: () => get<PublicAuthConfig>('/api/auth/config'),
    retry: false,
    staleTime: 5 * 60 * 1000,
  })
}
