import { IconCircleCheck } from '@tabler/icons-react'

import { ItemWorkspace } from '../components/ItemWorkspace'

export function Completed() {
  return (
    <ItemWorkspace
      title="Completed"
      description="A record of what you have finished."
      emptyIcon={IconCircleCheck}
      emptyTitle="Nothing completed yet"
      emptyDescription="Tick an item off and it will show up here."
      baseFilters={{ statuses: ['completed'], sort: 'updated_at', order: 'desc' }}
    />
  )
}
