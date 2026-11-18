import { useEffect } from 'react'
import { Navigate, Outlet, useLocation } from 'react-router'
import { useQueryClient } from '@tanstack/react-query'
import { useCurrentUser } from '../api/auth'
import { FullPageLoader } from '../components/FullPageLoader'
import { clearToken, getToken } from './token'

/**
 * Validate the stored token against the backend before rendering a protected
 * route. A token in localStorage is not proof of a valid session: it may be
 * expired, revoked, or belong to a deleted user.
 */
export function RequireAuth() {
  const location = useLocation()
  const queryClient = useQueryClient()
  const token = getToken()
  const currentUser = useCurrentUser({ enabled: token !== null })

  useEffect(() => {
    if (currentUser.isError) {
      clearToken()
      queryClient.clear()
    }
  }, [currentUser.isError, queryClient])

  if (token === null || currentUser.isError) {
    return (
      <Navigate
        to="/login"
        replace
        state={{ from: `${location.pathname}${location.search}` }}
      />
    )
  }

  if (currentUser.isLoading) {
    return <FullPageLoader label="Checking your session…" />
  }

  return <Outlet />
}
