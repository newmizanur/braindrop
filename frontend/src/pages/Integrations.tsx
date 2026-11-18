import {
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
  IconBrandDiscord,
  IconBrandTelegram,
  IconSparkles,
} from '@tabler/icons-react'
import { PageHeader } from '../components/PageHeader'

const integrations = [
  {
    name: 'OpenRouter',
    icon: IconSparkles,
    color: 'indigo',
    description:
      'Optional AI parsing that turns unstructured capture into reviewable item suggestions. Manual capture always works without it.',
    milestone: 'Milestone 4',
  },
  {
    name: 'Telegram',
    icon: IconBrandTelegram,
    color: 'cyan',
    description:
      'Receive scheduled reminders and daily digests through a Telegram bot. Credentials are encrypted at rest.',
    milestone: 'Milestone 5',
  },
  {
    name: 'Discord',
    icon: IconBrandDiscord,
    color: 'violet',
    description:
      'Deliver reminders and digests through a Discord webhook. No bot gateway connection is required.',
    milestone: 'Milestone 5',
  },
]

export function Integrations() {
  return (
    <>
      <PageHeader
        title="Integrations"
        description="Optional services. BrainDrop is fully usable without any of them."
        action={<Badge variant="light">Milestones 4–5</Badge>}
      />

      <SimpleGrid cols={{ base: 1, md: 3 }} spacing="md">
        {integrations.map((integration) => (
          <Paper key={integration.name} withBorder p="lg" radius="lg">
            <Stack gap="sm" h="100%">
              <Group justify="space-between">
                <ThemeIcon
                  size={44}
                  radius="md"
                  variant="light"
                  color={integration.color}
                >
                  <integration.icon size={24} />
                </ThemeIcon>
                <Badge variant="light" color={integration.color}>
                  Planned
                </Badge>
              </Group>
              <Title order={3} size="h4">
                {integration.name}
              </Title>
              <Text size="sm" c="dimmed" style={{ flexGrow: 1 }}>
                {integration.description}
              </Text>
              <Button variant="default" disabled>
                Configure in {integration.milestone}
              </Button>
            </Stack>
          </Paper>
        ))}
      </SimpleGrid>

      <Text size="xs" c="dimmed" mt="lg">
        Integrations are opt-in. No credentials are requested or stored in this
        build.
      </Text>
    </>
  )
}
