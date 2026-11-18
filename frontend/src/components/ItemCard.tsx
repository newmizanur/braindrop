import {
  ActionIcon,
  Badge,
  Checkbox,
  Group,
  Menu,
  Paper,
  Stack,
  Text,
} from '@mantine/core'
import { IconDots, IconPencil, IconTrash } from '@tabler/icons-react'

import type { ItemResponse } from '../bindings/ItemResponse'
import {
  formatDueDate,
  formatTags,
  isOverdue,
  itemTypeColor,
  itemTypeLabel,
  priorityColor,
  priorityLabel,
  statusColor,
  statusLabel,
} from '../lib/items'

interface ItemCardProps {
  item: ItemResponse
  onToggle: (item: ItemResponse) => void
  onEdit: (item: ItemResponse) => void
  onDelete: (item: ItemResponse) => void
  busy?: boolean
}

export function ItemCard({
  item,
  onToggle,
  onEdit,
  onDelete,
  busy = false,
}: ItemCardProps) {
  const completed = item.status === 'completed'
  const overdue = isOverdue(item)

  return (
    <Paper withBorder p="sm" radius="md" data-testid="item-card">
      <Group align="flex-start" wrap="nowrap" gap="sm">
        <Checkbox
          mt={3}
          checked={completed}
          disabled={busy}
          aria-label={
            completed ? `Reopen ${item.title}` : `Complete ${item.title}`
          }
          onChange={() => onToggle(item)}
        />

        <Stack gap={6} style={{ flex: 1, minWidth: 0 }}>
          <Group gap="xs" wrap="wrap">
            <Text
              fw={600}
              td={completed ? 'line-through' : undefined}
              c={completed ? 'dimmed' : undefined}
              style={{ overflowWrap: 'anywhere' }}
            >
              {item.title}
            </Text>
            <Badge size="xs" variant="light" color={itemTypeColor(item.type)}>
              {itemTypeLabel(item.type)}
            </Badge>
            {item.priority !== 'normal' && (
              <Badge
                size="xs"
                variant="light"
                color={priorityColor(item.priority)}
              >
                {priorityLabel(item.priority)}
              </Badge>
            )}
            {item.status !== 'inbox' && (
              <Badge
                size="xs"
                variant="dot"
                color={statusColor(item.status)}
              >
                {statusLabel(item.status)}
              </Badge>
            )}
          </Group>

          {item.content && (
            <Text
              size="sm"
              c="dimmed"
              lineClamp={2}
              className="prewrap"
            >
              {item.content}
            </Text>
          )}

          <Group gap="sm" wrap="wrap">
            {item.due_at && (
              <Text size="xs" c={overdue ? 'red' : 'dimmed'} fw={overdue ? 600 : 400}>
                Due {formatDueDate(item.due_at)}
                {overdue ? ' · overdue' : ''}
              </Text>
            )}
            {item.category && (
              <Badge size="xs" variant="outline" color="gray">
                {item.category}
              </Badge>
            )}
            {item.tags.length > 0 && (
              <Text size="xs" c="dimmed">
                #{formatTags(item.tags).replace(/, /g, ' #')}
              </Text>
            )}
          </Group>
        </Stack>

        <Menu position="bottom-end" withArrow>
          <Menu.Target>
            <ActionIcon
              variant="subtle"
              color="gray"
              aria-label={`Actions for ${item.title}`}
            >
              <IconDots size={18} />
            </ActionIcon>
          </Menu.Target>
          <Menu.Dropdown>
            <Menu.Item
              leftSection={<IconPencil size={16} />}
              onClick={() => onEdit(item)}
            >
              Edit
            </Menu.Item>
            <Menu.Item
              leftSection={<IconTrash size={16} />}
              color="red"
              onClick={() => onDelete(item)}
            >
              Delete
            </Menu.Item>
          </Menu.Dropdown>
        </Menu>
      </Group>
    </Paper>
  )
}
