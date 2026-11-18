import { Group, Text, ThemeIcon } from '@mantine/core'
import { IconBrain } from '@tabler/icons-react'

interface BrandProps {
  compact?: boolean
}

export function Brand({ compact = false }: BrandProps) {
  return (
    <Group gap="xs" wrap="nowrap">
      <ThemeIcon size={36} radius="md" variant="filled" color="indigo">
        <IconBrain size={22} />
      </ThemeIcon>
      {!compact && (
        <div>
          <Text fw={800} lh={1.15} size="lg">
            BrainDrop
          </Text>
          <Text size="xs" c="dimmed" lh={1.15}>
            Capture. Organize.
          </Text>
        </div>
      )}
    </Group>
  )
}
