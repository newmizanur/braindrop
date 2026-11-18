import {
  Alert,
  Badge,
  Divider,
  Group,
  Paper,
  SegmentedControl,
  Skeleton,
  Stack,
  Text,
  TextInput,
  Title,
  useMantineColorScheme,
} from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { PageHeader } from '../components/PageHeader'
import { useCurrentUser } from '../api/auth'

export function Settings() {
  const currentUser = useCurrentUser()
  const { colorScheme, setColorScheme } = useMantineColorScheme()

  return (
    <>
      <PageHeader
        title="Settings"
        description="Account, appearance and capture preferences."
        action={<Badge variant="light">Milestone 2–3</Badge>}
      />

      <Stack gap="lg" maw={720}>
        <Paper withBorder p="lg" radius="lg">
          <Title order={3} size="h4" mb="md">
            Account
          </Title>
          {currentUser.isLoading ? (
            <Stack gap="sm">
              <Skeleton height={36} />
              <Skeleton height={36} />
            </Stack>
          ) : (
            <Stack gap="md">
              <TextInput
                label="Name"
                value={currentUser.data?.name ?? ''}
                readOnly
              />
              <TextInput
                label="Email"
                value={currentUser.data?.email ?? ''}
                readOnly
              />
              <Text size="xs" c="dimmed">
                Accounts are created through the generated Loco auth flow.
                Milestone 2 adds an administrator task and configurable
                registration.
              </Text>
            </Stack>
          )}
        </Paper>

        <Paper withBorder p="lg" radius="lg">
          <Title order={3} size="h4" mb="md">
            Appearance
          </Title>
          <Stack gap="sm">
            <Text size="sm" fw={500}>
              Theme
            </Text>
            <SegmentedControl
              value={colorScheme}
              onChange={(value) =>
                setColorScheme(value as 'light' | 'dark' | 'auto')
              }
              data={[
                { value: 'light', label: 'Light' },
                { value: 'dark', label: 'Dark' },
                { value: 'auto', label: 'System' },
              ]}
            />
            <Text size="xs" c="dimmed">
              The preference is stored locally in this build. Milestone 3 syncs
              it to your account.
            </Text>
          </Stack>
        </Paper>

        <Paper withBorder p="lg" radius="lg">
          <Group justify="space-between" mb="md">
            <Title order={3} size="h4">
              Capture preferences
            </Title>
            <Badge variant="light">Milestone 3</Badge>
          </Group>
          <Stack gap="md">
            <TextInput
              label="Timezone"
              description="Used to resolve relative dates such as “tomorrow”."
              placeholder="UTC"
              disabled
            />
            <Divider />
            <Alert
              icon={<IconInfoCircle size={16} />}
              variant="light"
              color="gray"
            >
              Timezone, AI and notification preferences are stored per user in
              milestone 3 and used by the optional integrations in milestones
              4–5.
            </Alert>
          </Stack>
        </Paper>
      </Stack>
    </>
  )
}
