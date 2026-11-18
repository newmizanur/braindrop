import { IconSun } from '@tabler/icons-react'

import { useSettings } from '../api/settings'
import { ItemWorkspace } from '../components/ItemWorkspace'
import { todayBounds } from '../lib/items'

export function Today() {
  const settings = useSettings()
  const timezone = settings.data?.timezone ?? 'UTC'
  const { start, end } = todayBounds(timezone)

  return (
    <ItemWorkspace
      title="Today"
      description="Items due today, in your configured timezone."
      emptyIcon={IconSun}
      emptyTitle="Nothing due today"
      emptyDescription="Give yourself some breathing room, or plan something for later."
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
