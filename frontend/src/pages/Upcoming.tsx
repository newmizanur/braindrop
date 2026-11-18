import { Badge } from '@mantine/core'
import { IconCalendarEvent } from '@tabler/icons-react'
import { EmptyState } from '../components/EmptyState'
import { PageHeader } from '../components/PageHeader'

export function Upcoming() {
  return (
    <>
      <PageHeader
        title="Upcoming"
        description="The next few days and weeks at a glance."
        action={<Badge variant="light">Milestone 3</Badge>}
      />
      <EmptyState
        icon={IconCalendarEvent}
        title="No upcoming items"
        description="Scheduled reminders and due dates will appear here once items and notification scheduling are implemented."
      />
    </>
  )
}
