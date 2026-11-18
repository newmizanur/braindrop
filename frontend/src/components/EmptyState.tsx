import type { ReactNode } from 'react'
import { Paper, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import type { TablerIcon } from './PageHeader'

interface EmptyStateProps {
  icon: TablerIcon
  title: string
  description: string
  action?: ReactNode
}

export function EmptyState({ icon: Icon, title, description, action }: EmptyStateProps) {
  return (
    <Paper withBorder p="xl">
      <Stack align="center" gap="sm" py="lg">
        <ThemeIcon size={56} radius="xl" variant="light" color="indigo">
          <Icon size={28} />
        </ThemeIcon>
        <Title order={3} size="h4" ta="center">
          {title}
        </Title>
        <Text c="dimmed" ta="center" maw={440}>
          {description}
        </Text>
        {action}
      </Stack>
    </Paper>
  )
}
