import { useMemo, useState } from 'react'
import {
  Alert,
  Button,
  Divider,
  Group,
  Paper,
  SegmentedControl,
  Select,
  Skeleton,
  Stack,
  Text,
  TextInput,
  Title,
  useMantineColorScheme,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconDeviceFloppy, IconInfoCircle } from '@tabler/icons-react'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemType } from '../bindings/ItemType'
import type { SettingsResponse } from '../bindings/SettingsResponse'
import type { Theme } from '../bindings/Theme'
import { useCurrentUser } from '../api/auth'
import { useSettings, useUpdateSettings } from '../api/settings'
import { PageHeader } from '../components/PageHeader'
import {
  ITEM_PRIORITIES,
  ITEM_TYPES,
  itemTypeLabel,
  priorityLabel,
} from '../lib/items'

const typeOptions = ITEM_TYPES.map((value) => ({
  value,
  label: itemTypeLabel(value),
}))
const priorityOptions = ITEM_PRIORITIES.map((value) => ({
  value,
  label: priorityLabel(value),
}))
const weekStartOptions = [
  'Sunday',
  'Monday',
  'Tuesday',
  'Wednesday',
  'Thursday',
  'Friday',
  'Saturday',
].map((label, value) => ({ value: String(value), label }))

function useTimezoneOptions(current: string) {
  return useMemo(() => {
    const intl = Intl as unknown as {
      supportedValuesOf?: (key: string) => string[]
    }
    const supported = intl.supportedValuesOf?.('timeZone') ?? []
    const zones = new Set<string>(['UTC', ...supported, current])
    return Array.from(zones)
      .sort()
      .map((zone) => ({ value: zone, label: zone }))
  }, [current])
}

interface SettingsFormProps {
  settings: SettingsResponse
}

function SettingsForm({ settings }: SettingsFormProps) {
  const updateSettings = useUpdateSettings()
  const { setColorScheme } = useMantineColorScheme()

  const [timezone, setTimezone] = useState(settings.timezone)
  const [theme, setTheme] = useState<Theme>(settings.theme)
  const [defaultType, setDefaultType] = useState<ItemType>(
    settings.default_item_type,
  )
  const [defaultPriority, setDefaultPriority] = useState<ItemPriority>(
    settings.default_priority,
  )
  const [weekStart, setWeekStart] = useState(String(settings.week_start))

  const timezoneOptions = useTimezoneOptions(timezone)

  async function handleSave(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault()
    try {
      const saved = await updateSettings.mutateAsync({
        timezone,
        theme,
        default_item_type: defaultType,
        default_priority: defaultPriority,
        week_start: Number(weekStart),
      })
      setColorScheme(saved.theme)
      notifications.show({
        title: 'Settings saved',
        message: 'Your preferences were updated.',
        color: 'teal',
      })
    } catch (error) {
      notifications.show({
        title: 'Could not save settings',
        message: error instanceof Error ? error.message : 'Unknown error',
        color: 'red',
      })
    }
  }

  return (
    <form onSubmit={handleSave}>
      <Stack gap="lg">
        <div>
          <Title order={3} size="h4" mb="xs">
            Preferences
          </Title>
          <Text size="sm" c="dimmed">
            These values are stored with your account and used across devices.
          </Text>
        </div>

        <Select
          label="Timezone"
          description="Used for today, upcoming and dashboard day boundaries."
          data={timezoneOptions}
          value={timezone}
          onChange={(value) => value && setTimezone(value)}
          searchable
          allowDeselect={false}
        />

        <div>
          <Text size="sm" fw={500} mb={6}>
            Theme
          </Text>
          <SegmentedControl
            fullWidth
            value={theme}
            onChange={(value) => setTheme(value as Theme)}
            data={[
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
              { value: 'auto', label: 'System' },
            ]}
          />
        </div>

        <Group grow align="flex-start">
          <Select
            label="Default item type"
            data={typeOptions}
            value={defaultType}
            onChange={(value) => value && setDefaultType(value as ItemType)}
            allowDeselect={false}
          />
          <Select
            label="Default priority"
            data={priorityOptions}
            value={defaultPriority}
            onChange={(value) =>
              value && setDefaultPriority(value as ItemPriority)
            }
            allowDeselect={false}
          />
        </Group>

        <Select
          label="Week starts on"
          data={weekStartOptions}
          value={weekStart}
          onChange={(value) => value && setWeekStart(value)}
          allowDeselect={false}
          maw={220}
        />

        <Divider />

        <Group justify="space-between" align="center">
          <Alert
            icon={<IconInfoCircle size={16} />}
            variant="light"
            color="gray"
            p="xs"
          >
            <Text size="xs">
              AI and notification integrations stay optional. Configure AI and
              credentials on the Integrations page.
            </Text>
          </Alert>
          <Button
            type="submit"
            leftSection={<IconDeviceFloppy size={16} />}
            loading={updateSettings.isPending}
          >
            Save
          </Button>
        </Group>
      </Stack>
    </form>
  )
}

export function Settings() {
  const currentUser = useCurrentUser()
  const settings = useSettings()

  return (
    <>
      <PageHeader
        title="Settings"
        description="Account, appearance and capture preferences."
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
                Accounts are created by an administrator with{' '}
                <code>admin:create</code>, or through public registration when
                the instance enables it.
              </Text>
            </Stack>
          )}
        </Paper>

        <Paper withBorder p="lg" radius="lg">
          {settings.isLoading && (
            <Stack gap="md">
              <Skeleton height={24} width="40%" />
              <Skeleton height={36} />
              <Skeleton height={36} />
              <Skeleton height={36} />
            </Stack>
          )}
          {settings.isError && (
            <Alert color="red" variant="light">
              Could not load your settings: {settings.error.message}
            </Alert>
          )}
          {settings.data && (
            <SettingsForm
              key={JSON.stringify(settings.data)}
              settings={settings.data}
            />
          )}
        </Paper>
      </Stack>
    </>
  )
}
