import { IconCalendarEvent } from '@tabler/icons-react'

import { useSettings } from '../api/settings'
import { ItemWorkspace } from '../components/ItemWorkspace'
import { upcomingBounds } from '../lib/items'

export function Upcoming() {
  const settings = useSettings()
  const timezone = settings.data?.timezone ?? 'UTC'
  const { start, end } = upcomingBounds(timezone)

  return (
    <ItemWorkspace
      title="Upcoming"
      description="The next seven days at a glance."
      emptyIcon={IconCalendarEvent}
      emptyTitle="No upcoming items"
      emptyDescription="Items with a due date in the next week will appear here."
      baseFilters={{
        dueFrom: start,
        dueTo: end,
        statuses: ['inbox', 'pending'],
        sort: 'due_at',
        order: 'asc',
      }}
    />
  )
}
