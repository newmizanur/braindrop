import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router'
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
import { AuthLayout } from '../components/AuthLayout'

export function Register() {
  const [name, setName] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [isPending, setIsPending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [isDone, setIsDone] = useState(false)

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setError(null)
    setIsPending(true)
    try {
      // `register` answers 200 with a null body whether or not the address was
      // already taken — deliberately, so the endpoint cannot be used to probe
      // for accounts. There is nothing to read back, and the success copy has
      // to stay neutral for the same reason.
      await post<null>('/api/auth/register', { name, email, password })
      setIsDone(true)
    } catch (err) {
      setError(
        err instanceof ApiClientError ? err.message : 'Failed to register',
      )
    } finally {
      setIsPending(false)
    }
  }

  if (isDone) {
    return (
      <AuthLayout
        title="Check your email"
        subtitle="One more step before you can log in."
      >
        <Alert
          icon={<IconMailCheck size={18} />}
          color="teal"
          variant="light"
          title="Verification link sent"
        >
          If that address is available, a verification link is on its way. You
          need to verify before you can log in.
        </Alert>
        <Button component={Link} to="/login" variant="default" fullWidth>
          Back to log in
        </Button>
      </AuthLayout>
    )
  }

  return (
    <AuthLayout
      title="Create your account"
      subtitle="A private inbox for everything on your mind."
    >
      <form onSubmit={handleSubmit}>
        <Stack gap="md">
          <TextInput
            label="Name"
            id="name"
            name="name"
            placeholder="Ada Lovelace"
            autoComplete="name"
            required
            minLength={2}
            value={name}
            onChange={(event) => setName(event.currentTarget.value)}
          />
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
            placeholder="Choose a password"
            autoComplete="new-password"
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

          <Button type="submit" loading={isPending} fullWidth>
            {isPending ? 'Creating account…' : 'Create account'}
          </Button>
        </Stack>
      </form>

      <Text size="sm" c="dimmed" ta="center">
        Already have an account?{' '}
        <Anchor component={Link} to="/login" fw={600}>
          Log in
        </Anchor>
      </Text>
    </AuthLayout>
  )
}
