import { Badge } from '@mantine/core'
import { IconSun } from '@tabler/icons-react'
import { EmptyState } from '../components/EmptyState'
import { PageHeader } from '../components/PageHeader'

export function Today() {
  return (
    <>
      <PageHeader
        title="Today"
        description="Items due today, in your timezone."
        action={<Badge variant="light">Milestone 3</Badge>}
      />
      <EmptyState
        icon={IconSun}
        title="Nothing due today"
        description="Due dates and timezone-aware grouping arrive with the items API in milestone 3."
      />
    </>
  )
}
