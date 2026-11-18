import {
  Badge,
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
  IconCircleCheck,
  IconClock,
  IconInbox,
} from '@tabler/icons-react'
import { EmptyState } from '../components/EmptyState'
import { PageHeader } from '../components/PageHeader'

const stats = [
  { label: 'Inbox', icon: IconInbox, color: 'indigo' },
  { label: 'Due today', icon: IconClock, color: 'orange' },
  { label: 'Overdue', icon: IconAlertTriangle, color: 'red' },
  { label: 'Completed this week', icon: IconCircleCheck, color: 'teal' },
]

export function Dashboard() {
  return (
    <>
      <PageHeader
        title="Dashboard"
        description="A calm overview of what needs your attention."
        action={<Badge variant="light">Milestone 3</Badge>}
      />

      <SimpleGrid cols={{ base: 1, xs: 2, md: 4 }} spacing="md" mb="xl">
        {stats.map((stat) => (
          <Paper key={stat.label} withBorder p="md" radius="lg">
            <Group justify="space-between" align="flex-start">
              <div>
                <Text size="xs" c="dimmed" tt="uppercase" fw={700}>
                  {stat.label}
                </Text>
                <Title order={2} mt={6}>
                  —
                </Title>
              </div>
              <ThemeIcon variant="light" color={stat.color} radius="md">
                <stat.icon size={18} />
              </ThemeIcon>
            </Group>
          </Paper>
        ))}
      </SimpleGrid>

      <EmptyState
        icon={IconInbox}
        title="Nothing to show yet"
        description="Dashboard counts come from /api/stats in milestone 3. Until then, use Quick capture in the header to see the capture flow."
        action={
          <Stack align="center" gap="xs">
            <Text size="xs" c="dimmed">
              Manual capture will work with no AI provider configured.
            </Text>
            <Badge variant="dot" color="indigo">
              Bootstrap build
            </Badge>
          </Stack>
        }
      />
    </>
  )
}
