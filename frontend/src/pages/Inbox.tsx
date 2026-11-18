import { Badge } from '@mantine/core'
import { IconInbox } from '@tabler/icons-react'
import { EmptyState } from '../components/EmptyState'
import { PageHeader } from '../components/PageHeader'

export function Inbox() {
  return (
    <>
      <PageHeader
        title="Inbox"
        description="Everything you capture lands here until you triage it."
        action={<Badge variant="light">Milestone 3</Badge>}
      />
      <EmptyState
        icon={IconInbox}
        title="Your inbox is empty"
        description="Quick capture from the header will create items here once the items API lands in milestone 3."
      />
    </>
  )
}
