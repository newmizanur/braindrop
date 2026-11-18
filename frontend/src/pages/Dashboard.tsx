import { Link } from 'react-router'
import {
  Alert,
  Badge,
  Button,
  Group,
  Paper,
  SimpleGrid,
  Stack,
  Text,
  ThemeIcon,
  Title,
} from '@mantine/core'
import {
  IconAlertTriangle,
  IconCalendarEvent,
  IconCircleCheck,
  IconClock,
  IconInbox,
  IconNotes,
  IconSparkles,
} from '@tabler/icons-react'

import { useItems } from '../api/items'
import { useSettings } from '../api/settings'
import { useStats } from '../api/stats'
import { PageHeader } from '../components/PageHeader'
import {
  formatDueDate,
  isOpen,
  todayBounds,
} from '../lib/items'

interface StatCardProps {
  label: string
  value: number | undefined
  icon: typeof IconInbox
  color: string
  to: string
}

function StatCard({ label, value, icon: Icon, color, to }: StatCardProps) {
  return (
    <Paper
      withBorder
      p="md"
      radius="lg"
      component={Link}
      to={to}
      style={{ textDecoration: 'none' }}
    >
      <Group justify="space-between" align="flex-start">
        <div>
          <Text size="xs" c="dimmed" tt="uppercase" fw={700}>
            {label}
          </Text>
          <Title order={2} mt={6}>
            {value ?? '—'}
          </Title>
        </div>
        <ThemeIcon variant="light" color={color} radius="md">
          <Icon size={18} />
        </ThemeIcon>
      </Group>
    </Paper>
  )
}

export function Dashboard() {
  const stats = useStats()
  const settings = useSettings()
  const timezone = settings.data?.timezone ?? 'UTC'
  const { start, end } = todayBounds(timezone)

  const dueToday = useItems({
    dueFrom: start,
    dueTo: end,
    statuses: ['inbox', 'pending'],
    sort: 'due_at',
    order: 'asc',
    pageSize: 5,
  })

  const recent = useItems({
    statuses: ['inbox'],
    sort: 'created_at',
    order: 'desc',
    pageSize: 5,
  })

  return (
    <>
      <PageHeader
        title="Dashboard"
        description="A calm overview of what needs your attention."
      />

      {stats.isError && (
        <Alert color="red" variant="light" mb="md">
          Could not load your statistics: {stats.error.message}
        </Alert>
      )}

      <SimpleGrid cols={{ base: 1, xs: 2, md: 4 }} spacing="md" mb="lg">
        <StatCard
          label="Inbox"
          value={stats.data?.inbox}
          icon={IconInbox}
          color="indigo"
          to="/inbox"
        />
        <StatCard
          label="Due today"
          value={stats.data?.due_today}
          icon={IconClock}
          color="orange"
          to="/today"
        />
        <StatCard
          label="Overdue"
          value={stats.data?.overdue}
          icon={IconAlertTriangle}
          color="red"
          to="/items"
        />
        <StatCard
          label="Completed this week"
          value={stats.data?.completed_week}
          icon={IconCircleCheck}
          color="teal"
          to="/completed"
        />
      </SimpleGrid>

      <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md" mb="lg">
        <Paper withBorder p="lg" radius="lg">
          <Group justify="space-between" mb="md">
            <Group gap="xs">
              <IconClock size={18} />
              <Title order={3} size="h4">
                Due today
              </Title>
            </Group>
            <Button variant="subtle" size="xs" component={Link} to="/today">
              View all
            </Button>
          </Group>
          {dueToday.data && dueToday.data.items.length > 0 ? (
            <Stack gap="xs">
              {dueToday.data.items.map((item) => (
                <Paper key={item.pid} withBorder p="xs" radius="sm">
                  <Group justify="space-between" wrap="nowrap">
                    <Text size="sm" lineClamp={1}>
                      {item.title}
                    </Text>
                    {item.due_at && (
                      <Text size="xs" c="dimmed" style={{ whiteSpace: 'nowrap' }}>
                        {formatDueDate(item.due_at)}
                      </Text>
                    )}
                  </Group>
                </Paper>
              ))}
            </Stack>
          ) : (
            <Text size="sm" c="dimmed">
              {dueToday.isLoading ? 'Loading…' : 'Nothing due today. Enjoy the space.'}
            </Text>
          )}
        </Paper>

        <Paper withBorder p="lg" radius="lg">
          <Group justify="space-between" mb="md">
            <Group gap="xs">
              <IconInbox size={18} />
              <Title order={3} size="h4">
                Recently captured
              </Title>
            </Group>
            <Button variant="subtle" size="xs" component={Link} to="/inbox">
              Open inbox
            </Button>
          </Group>
          {recent.data && recent.data.items.length > 0 ? (
            <Stack gap="xs">
              {recent.data.items.filter(isOpen).map((item) => (
                <Paper key={item.pid} withBorder p="xs" radius="sm">
                  <Group justify="space-between" wrap="nowrap">
                    <Text size="sm" lineClamp={1}>
                      {item.title}
                    </Text>
                    <Badge size="xs" variant="light" color="indigo">
                      {item.type}
                    </Badge>
                  </Group>
                </Paper>
              ))}
            </Stack>
          ) : (
            <Text size="sm" c="dimmed">
              {recent.isLoading
                ? 'Loading…'
                : 'Your inbox is empty. Capture something to get started.'}
            </Text>
          )}
        </Paper>
      </SimpleGrid>

      <SimpleGrid cols={{ base: 2, sm: 4 }} spacing="md">
        <SummaryTile
          icon={IconCalendarEvent}
          label="Upcoming"
          value={stats.data?.upcoming}
        />
        <SummaryTile
          icon={IconSparkles}
          label="Ideas"
          value={stats.data?.ideas}
        />
        <SummaryTile
          icon={IconNotes}
          label="Notes"
          value={stats.data?.notes}
        />
        <SummaryTile
          icon={IconCircleCheck}
          label="Completed today"
          value={stats.data?.completed_today}
        />
      </SimpleGrid>
    </>
  )
}

function SummaryTile({
  icon: Icon,
  label,
  value,
}: {
  icon: typeof IconInbox
  label: string
  value: number | undefined
}) {
  return (
    <Paper withBorder p="sm" radius="md">
      <Group gap="xs" wrap="nowrap">
        <ThemeIcon variant="light" color="gray" radius="sm" size="sm">
          <Icon size={14} />
        </ThemeIcon>
        <div>
          <Text size="xs" c="dimmed">
            {label}
          </Text>
          <Text fw={600}>{value ?? '—'}</Text>
        </div>
      </Group>
    </Paper>
  )
}
