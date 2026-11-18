import { IconList } from '@tabler/icons-react'

import { ItemWorkspace } from '../components/ItemWorkspace'

export function AllItems() {
  return (
    <ItemWorkspace
      title="All items"
      description="Tasks, notes, ideas and reminders in one list."
      emptyIcon={IconList}
      emptyTitle="No items yet"
      emptyDescription="Create your first item, or capture a thought from the header."
      showStatusFilter
      baseFilters={{ sort: 'created_at', order: 'desc' }}
    />
  )
}
