import { Paper, Skeleton, Stack } from '@mantine/core'

import type { ItemResponse } from '../bindings/ItemResponse'
import type { TablerIcon } from './PageHeader'
import { EmptyState } from './EmptyState'
import { ItemCard } from './ItemCard'

interface ItemListProps {
  items: ItemResponse[]
  isLoading: boolean
  busyPid?: string
  emptyIcon: TablerIcon
  emptyTitle: string
  emptyDescription: string
  onToggle: (item: ItemResponse) => void
  onEdit: (item: ItemResponse) => void
  onDelete: (item: ItemResponse) => void
}

export function ItemList({
  items,
  isLoading,
  busyPid,
  emptyIcon,
  emptyTitle,
  emptyDescription,
  onToggle,
  onEdit,
  onDelete,
}: ItemListProps) {
  if (isLoading) {
    return (
      <Stack gap="sm" aria-busy="true" aria-label="Loading items">
        {[0, 1, 2].map((index) => (
          <Paper key={index} withBorder p="sm" radius="md">
            <Skeleton height={18} width="45%" mb={8} />
            <Skeleton height={12} width="70%" />
          </Paper>
        ))}
      </Stack>
    )
  }

  if (items.length === 0) {
    return (
      <EmptyState
        icon={emptyIcon}
        title={emptyTitle}
        description={emptyDescription}
      />
    )
  }

  return (
    <Stack gap="sm" data-testid="item-list">
      {items.map((item) => (
        <ItemCard
          key={item.pid}
          item={item}
          busy={busyPid === item.pid}
          onToggle={onToggle}
          onEdit={onEdit}
          onDelete={onDelete}
        />
      ))}
    </Stack>
  )
}
