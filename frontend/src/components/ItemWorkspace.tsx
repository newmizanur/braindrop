import { useState } from 'react'
import { Button, Group, Pagination, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconPlus } from '@tabler/icons-react'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemResponse } from '../bindings/ItemResponse'
import type { ItemType } from '../bindings/ItemType'
import type { ItemInput, ItemUpdate, ItemFilters } from '../api/items'
import {
  useCreateItems,
  useDeleteItem,
  useItems,
  useUpdateItem,
} from '../api/items'
import { useSettings } from '../api/settings'
import { ConfirmDialog } from './ConfirmDialog'
import { ItemEditor } from './ItemEditor'
import { ItemFiltersBar, type FilterState } from './ItemFiltersBar'
import { ItemList } from './ItemList'
import { PageHeader, type TablerIcon } from './PageHeader'

const PAGE_SIZE = 20

interface ItemWorkspaceProps {
  title: string
  description: string
  emptyIcon: TablerIcon
  emptyTitle: string
  emptyDescription: string
  baseFilters?: ItemFilters
  showFilters?: boolean
  showStatusFilter?: boolean
  createDefaults?: { type?: ItemType; priority?: ItemPriority }
}

export function ItemWorkspace({
  title,
  description,
  emptyIcon,
  emptyTitle,
  emptyDescription,
  baseFilters,
  showFilters = true,
  showStatusFilter = false,
  createDefaults,
}: ItemWorkspaceProps) {
  const [filters, setFilters] = useState<FilterState>({
    q: '',
    type: null,
    priority: null,
    statuses: [],
    sort: baseFilters?.sort ?? 'created_at',
    order: baseFilters?.order ?? 'desc',
  })
  const [page, setPage] = useState(1)
  const [editorOpen, setEditorOpen] = useState(false)
  const [editing, setEditing] = useState<ItemResponse | null>(null)
  const [deleting, setDeleting] = useState<ItemResponse | null>(null)

  const queryFilters: ItemFilters = {
    ...baseFilters,
    q: filters.q || undefined,
    type: filters.type ?? undefined,
    priority: filters.priority ?? undefined,
    statuses: showStatusFilter
      ? filters.statuses.length > 0
        ? filters.statuses
        : undefined
      : baseFilters?.statuses,
    sort: filters.sort,
    order: filters.order,
    page,
    pageSize: PAGE_SIZE,
  }

  const items = useItems(queryFilters)
  const settings = useSettings()
  const createItems = useCreateItems()
  const updateItem = useUpdateItem()
  const deleteItem = useDeleteItem()

  // Account defaults apply unless the page asks for something specific.
  const editorDefaults = createDefaults ?? {
    type: settings.data?.default_item_type,
    priority: settings.data?.default_priority,
  }

  const totalPages = items.data?.total_pages ?? 0
  const hasActiveFilters =
    filters.q.trim().length > 0 ||
    filters.type !== null ||
    filters.priority !== null ||
    filters.statuses.length > 0

  function handleFilters(next: FilterState) {
    setFilters(next)
    setPage(1)
  }

  function openCreate() {
    setEditing(null)
    setEditorOpen(true)
  }

  function openEdit(item: ItemResponse) {
    setEditing(item)
    setEditorOpen(true)
  }

  async function handleSubmit(input: ItemInput, pid?: string) {
    if (pid) {
      const update: ItemUpdate = {
        ...input,
        type: input.type ?? undefined,
        status: input.status ?? undefined,
        priority: input.priority ?? undefined,
      }
      await updateItem.mutateAsync({ pid, update })
      notifications.show({
        title: 'Item updated',
        message: 'Your changes were saved.',
        color: 'teal',
      })
    } else {
      await createItems.mutateAsync([input])
      notifications.show({
        title: 'Item created',
        message: 'Added to your list.',
        color: 'teal',
      })
    }
  }

  function handleToggle(item: ItemResponse) {
    const status = item.status === 'completed' ? 'inbox' : 'completed'
    updateItem.mutate(
      { pid: item.pid, update: { status } },
      {
        onError: (error) => {
          notifications.show({
            title: 'Could not update the item',
            message: error.message,
            color: 'red',
          })
        },
      },
    )
  }

  async function handleDelete() {
    if (!deleting) {
      return
    }
    try {
      await deleteItem.mutateAsync(deleting.pid)
      notifications.show({
        title: 'Item deleted',
        message: `“${deleting.title}” was removed.`,
        color: 'teal',
      })
      setDeleting(null)
    } catch (error) {
      notifications.show({
        title: 'Could not delete the item',
        message: error instanceof Error ? error.message : 'Unknown error',
        color: 'red',
      })
    }
  }

  return (
    <>
      <PageHeader
        title={title}
        description={description}
        action={
          <Button leftSection={<IconPlus size={16} />} onClick={openCreate}>
            New item
          </Button>
        }
      />

      <Stack gap="md">
        {showFilters && (
          <ItemFiltersBar
            value={filters}
            onChange={handleFilters}
            showStatus={showStatusFilter}
          />
        )}

        <ItemList
          items={items.data?.items ?? []}
          isLoading={items.isLoading}
          busyPid={updateItem.isPending ? updateItem.variables?.pid : undefined}
          emptyIcon={emptyIcon}
          emptyTitle={hasActiveFilters ? 'No matching items' : emptyTitle}
          emptyDescription={
            hasActiveFilters
              ? 'Try a different search or clear the filters.'
              : emptyDescription
          }
          onToggle={handleToggle}
          onEdit={openEdit}
          onDelete={setDeleting}
        />

        {items.isError && (
          <Text c="red" size="sm">
            {items.error.message}
          </Text>
        )}

        {totalPages > 1 && (
          <Group justify="center" mt="sm">
            <Pagination
              value={page}
              onChange={setPage}
              total={totalPages}
              size="sm"
            />
          </Group>
        )}

        {items.data && items.data.total_items > 0 && (
          <Text size="xs" c="dimmed" ta="center">
            {items.data.total_items} item
            {items.data.total_items === 1 ? '' : 's'}
          </Text>
        )}
      </Stack>

      {editorOpen && (
        <ItemEditor
          key={editing?.pid ?? 'new'}
          opened
          onClose={() => setEditorOpen(false)}
          item={editing}
          defaults={editorDefaults}
          onSubmit={handleSubmit}
        />
      )}

      <ConfirmDialog
        opened={deleting !== null}
        title="Delete item"
        message={`Delete “${deleting?.title ?? ''}”? This cannot be undone.`}
        confirmLabel="Delete"
        loading={deleteItem.isPending}
        onConfirm={handleDelete}
        onCancel={() => setDeleting(null)}
      />
    </>
  )
}
