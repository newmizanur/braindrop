/* eslint-disable react-refresh/only-export-components */
import { lazy, Suspense, type ReactNode } from 'react'
import { createBrowserRouter, Navigate } from 'react-router'

import { App } from './App'
import { Login } from './auth/Login'
import { Register } from './auth/Register'
import { RequireAuth } from './auth/RequireAuth'
import { FullPageLoader } from './components/FullPageLoader'

// Route-level code splitting keeps the initial bundle small; each workspace
// page (and its date pickers) loads on first visit.
const Dashboard = lazy(() =>
  import('./pages/Dashboard').then((module) => ({ default: module.Dashboard })),
)
const Inbox = lazy(() =>
  import('./pages/Inbox').then((module) => ({ default: module.Inbox })),
)
const Today = lazy(() =>
  import('./pages/Today').then((module) => ({ default: module.Today })),
)
const Upcoming = lazy(() =>
  import('./pages/Upcoming').then((module) => ({ default: module.Upcoming })),
)
const AllItems = lazy(() =>
  import('./pages/AllItems').then((module) => ({ default: module.AllItems })),
)
const Completed = lazy(() =>
  import('./pages/Completed').then((module) => ({ default: module.Completed })),
)
const Settings = lazy(() =>
  import('./pages/Settings').then((module) => ({ default: module.Settings })),
)
const Integrations = lazy(() =>
  import('./pages/Integrations').then((module) => ({
    default: module.Integrations,
  })),
)

function LazyPage({ children }: { children: ReactNode }) {
  return (
    <Suspense fallback={<FullPageLoader label="Loading…" />}>
      {children}
    </Suspense>
  )
}

export const router = createBrowserRouter([
  { path: '/login', element: <Login /> },
  { path: '/register', element: <Register /> },
  {
    element: <RequireAuth />,
    children: [
      {
        path: '/',
        element: <App />,
        children: [
          { index: true, element: <Navigate to="/dashboard" replace /> },
          {
            path: 'dashboard',
            element: (
              <LazyPage>
                <Dashboard />
              </LazyPage>
            ),
          },
          {
            path: 'inbox',
            element: (
              <LazyPage>
                <Inbox />
              </LazyPage>
            ),
          },
          {
            path: 'today',
            element: (
              <LazyPage>
                <Today />
              </LazyPage>
            ),
          },
          {
            path: 'upcoming',
            element: (
              <LazyPage>
                <Upcoming />
              </LazyPage>
            ),
          },
          {
            path: 'items',
            element: (
              <LazyPage>
                <AllItems />
              </LazyPage>
            ),
          },
          {
            path: 'completed',
            element: (
              <LazyPage>
                <Completed />
              </LazyPage>
            ),
          },
          {
            path: 'settings',
            element: (
              <LazyPage>
                <Settings />
              </LazyPage>
            ),
          },
          {
            path: 'integrations',
            element: (
              <LazyPage>
                <Integrations />
              </LazyPage>
            ),
          },
        ],
      },
    ],
  },
  { path: '*', element: <Navigate to="/dashboard" replace /> },
])
