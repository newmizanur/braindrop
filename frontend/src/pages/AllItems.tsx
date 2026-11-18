import { Badge } from '@mantine/core'
import { IconList } from '@tabler/icons-react'
import { EmptyState } from '../components/EmptyState'
import { PageHeader } from '../components/PageHeader'

export function AllItems() {
  return (
    <>
      <PageHeader
        title="All items"
        description="Tasks, notes, ideas and reminders in one list."
        action={<Badge variant="light">Milestone 3</Badge>}
      />
      <EmptyState
        icon={IconList}
        title="No items yet"
        description="Filtering, sorting, pagination and text search are implemented in milestone 3 together with the items API."
      />
    </>
  )
}
