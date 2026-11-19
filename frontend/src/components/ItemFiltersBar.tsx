import {
  Group,
  MultiSelect,
  Paper,
  SegmentedControl,
  Select,
  TextInput,
} from '@mantine/core'
import { useDebouncedValue } from '@mantine/hooks'
import { IconSearch } from '@tabler/icons-react'
import { useEffect, useState } from 'react'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemSort } from '../bindings/ItemSort'
import type { ItemStatus } from '../bindings/ItemStatus'
import type { ItemType } from '../bindings/ItemType'
import type { SortOrder } from '../bindings/SortOrder'
import {
  ITEM_PRIORITIES,
  ITEM_STATUSES,
  ITEM_TYPES,
  itemTypeLabel,
  priorityLabel,
  statusLabel,
} from '../lib/items'

export interface FilterState {
  q: string
  type: ItemType | null
  priority: ItemPriority | null
  statuses: ItemStatus[]
  sort: ItemSort
  order: SortOrder
}

interface ItemFiltersBarProps {
  value: FilterState
  onChange: (next: FilterState) => void
  showStatus?: boolean
}

const typeOptions = ITEM_TYPES.map((value) => ({
  value,
  label: itemTypeLabel(value),
}))
const priorityOptions = ITEM_PRIORITIES.map((value) => ({
  value,
  label: priorityLabel(value),
}))
const statusOptions = ITEM_STATUSES.map((value) => ({
  value,
  label: statusLabel(value),
}))

const sortOptions: { value: ItemSort; label: string }[] = [
  { value: 'created_at', label: 'Created' },
  { value: 'updated_at', label: 'Updated' },
  { value: 'due_at', label: 'Due date' },
  { value: 'priority', label: 'Priority' },
  { value: 'title', label: 'Title' },
]

export function ItemFiltersBar({
  value,
  onChange,
  showStatus = false,
}: ItemFiltersBarProps) {
  const [search, setSearch] = useState(value.q)
  const [debouncedSearch] = useDebouncedValue(search, 300)

  // Push the debounced value up only when it actually changes, so typing does
  // not trigger a request per keystroke.
  useEffect(() => {
    if (debouncedSearch !== value.q) {
      onChange({ ...value, q: debouncedSearch })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debouncedSearch])

  return (
    <Paper withBorder p="sm" radius="md" mb="md">
      <Group gap="sm" align="flex-end" wrap="wrap">
        <TextInput
          label="Search"
          placeholder="Title, notes or category"
          leftSection={<IconSearch size={16} />}
          value={search}
          onChange={(event) => setSearch(event.currentTarget.value)}
          style={{ flex: '1 1 220px' }}
          aria-label="Search items"
        />

        <Select
          label="Type"
          placeholder="All types"
          data={typeOptions}
          value={value.type}
          onChange={(next) =>
            onChange({ ...value, type: (next as ItemType | null) ?? null })
          }
          clearable
          style={{ width: 150 }}
        />

        <Select
          label="Priority"
          placeholder="Any priority"
          data={priorityOptions}
          value={value.priority}
          onChange={(next) =>
            onChange({
              ...value,
              priority: (next as ItemPriority | null) ?? null,
            })
          }
          clearable
          style={{ width: 150 }}
        />

        {showStatus && (
          <MultiSelect
            label="Status"
            placeholder="Any status"
            data={statusOptions}
            value={value.statuses}
            onChange={(next) =>
              onChange({ ...value, statuses: next as ItemStatus[] })
            }
            style={{ minWidth: 200, flex: '1 1 200px' }}
          />
        )}

        <Select
          label="Sort"
          data={sortOptions}
          value={value.sort}
          onChange={(next) =>
            next && onChange({ ...value, sort: next as ItemSort })
          }
          allowDeselect={false}
          style={{ width: 150 }}
        />

        <div>
          <SegmentedControl
            size="xs"
            value={value.order}
            onChange={(next) =>
              onChange({ ...value, order: next as SortOrder })
            }
            data={[
              { value: 'asc', label: 'Ascending' },
              { value: 'desc', label: 'Descending' },
            ]}
            aria-label="Sort direction"
          />
        </div>
      </Group>
    </Paper>
  )
}
