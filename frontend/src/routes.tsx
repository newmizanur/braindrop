import { createBrowserRouter, Navigate } from 'react-router'
import { App } from './App'
import { Login } from './auth/Login'
import { Register } from './auth/Register'
import { RequireAuth } from './auth/RequireAuth'
import { Dashboard } from './pages/Dashboard'
import { Inbox } from './pages/Inbox'
import { Today } from './pages/Today'
import { Upcoming } from './pages/Upcoming'
import { AllItems } from './pages/AllItems'
import { Completed } from './pages/Completed'
import { Settings } from './pages/Settings'
import { Integrations } from './pages/Integrations'

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
          { path: 'dashboard', element: <Dashboard /> },
          { path: 'inbox', element: <Inbox /> },
          { path: 'today', element: <Today /> },
          { path: 'upcoming', element: <Upcoming /> },
          { path: 'items', element: <AllItems /> },
          { path: 'completed', element: <Completed /> },
          { path: 'settings', element: <Settings /> },
          { path: 'integrations', element: <Integrations /> },
        ],
      },
    ],
  },
  { path: '*', element: <Navigate to="/dashboard" replace /> },
])
