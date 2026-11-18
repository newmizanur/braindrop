import type { ReactNode } from 'react'
import { Group, Text, Title } from '@mantine/core'
import type { Icon } from '@tabler/icons-react'

interface PageHeaderProps {
  title: string
  description?: string
  action?: ReactNode
}

export function PageHeader({ title, description, action }: PageHeaderProps) {
  return (
    <Group justify="space-between" align="flex-start" gap="md" mb="lg" wrap="wrap">
      <div>
        <Title order={1} size="h2">
          {title}
        </Title>
        {description && (
          <Text c="dimmed" size="sm" mt={4}>
            {description}
          </Text>
        )}
      </div>
      {action}
    </Group>
  )
}

export type TablerIcon = Icon
