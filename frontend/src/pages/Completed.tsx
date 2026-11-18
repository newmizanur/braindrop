import { Badge } from '@mantine/core'
import { IconCircleCheck } from '@tabler/icons-react'
import { EmptyState } from '../components/EmptyState'
import { PageHeader } from '../components/PageHeader'

export function Completed() {
  return (
    <>
      <PageHeader
        title="Completed"
        description="A record of what you have finished."
        action={<Badge variant="light">Milestone 3</Badge>}
      />
      <EmptyState
        icon={IconCircleCheck}
        title="Nothing completed yet"
        description="Completing an item will move it here once item state transitions are implemented in milestone 3."
      />
    </>
  )
}
