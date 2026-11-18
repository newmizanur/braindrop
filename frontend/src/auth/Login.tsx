import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useLocation, useNavigate } from 'react-router'
import {
  Alert,
  Anchor,
  Button,
  PasswordInput,
  Stack,
  Text,
  TextInput,
} from '@mantine/core'
import { IconAlertCircle, IconMailCheck } from '@tabler/icons-react'
import { ApiClientError, post } from '../api/client'
import { usePublicAuthConfig } from '../api/config'
import { AuthLayout } from '../components/AuthLayout'
import { setToken } from './token'

interface LoginResponse {
  token: string
  pid: string
  name: string
  is_verified: boolean
}

export function Login() {
  const navigate = useNavigate()
  const location = useLocation()
  const authConfig = usePublicAuthConfig()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [isPending, setIsPending] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const requestedPath = (location.state as { from?: string } | null)?.from
  const destination =
    requestedPath && requestedPath.startsWith('/') ? requestedPath : '/dashboard'

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    setIsPending(true)
    try {
      const response = await post<LoginResponse>('/api/auth/login', {
        email,
        password,
      })
      setToken(response.token)
      navigate(destination, { replace: true })
    } catch (err) {
      setError(
        err instanceof ApiClientError ? err.message : 'Failed to log in',
      )
    } finally {
      setIsPending(false)
    }
  }

  return (
    <AuthLayout
      title="Welcome back"
      subtitle="Log in to pick up where you left off."
    >
      <form onSubmit={handleSubmit}>
        <Stack gap="md">
          <TextInput
            label="Email"
            id="email"
            name="email"
            type="email"
            placeholder="you@example.com"
            autoComplete="email"
            required
            value={email}
            onChange={(event) => setEmail(event.currentTarget.value)}
          />
          <PasswordInput
            label="Password"
            id="password"
            name="password"
            placeholder="Your password"
            autoComplete="current-password"
            required
            value={password}
            onChange={(event) => setPassword(event.currentTarget.value)}
          />

          {error && (
            <Alert
              icon={<IconAlertCircle size={16} />}
              color="red"
              variant="light"
              role="alert"
            >
              {error}
            </Alert>
          )}

          {authConfig.data?.require_email_verification && (
            <Alert
              icon={<IconMailCheck size={16} />}
              color="blue"
              variant="light"
            >
              Email verification is required. Check your inbox for the
              verification link before logging in.
            </Alert>
          )}

          <Button type="submit" loading={isPending} fullWidth>
            {isPending ? 'Logging in…' : 'Log in'}
          </Button>
        </Stack>
      </form>

      {authConfig.data?.allow_registration ? (
        <Text size="sm" c="dimmed" ta="center">
          Don&apos;t have an account?{' '}
          <Anchor component={Link} to="/register" fw={600}>
            Sign up
          </Anchor>
        </Text>
      ) : (
        authConfig.data && (
          <Text size="sm" c="dimmed" ta="center">
            Public registration is disabled. Ask an administrator to create your
            account.
          </Text>
        )
      )}
    </AuthLayout>
  )
}
