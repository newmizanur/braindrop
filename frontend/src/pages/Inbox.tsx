import { IconInbox } from '@tabler/icons-react'

import { ItemWorkspace } from '../components/ItemWorkspace'

export function Inbox() {
  return (
    <ItemWorkspace
      title="Inbox"
      description="Everything you capture lands here until you triage it."
      emptyIcon={IconInbox}
      emptyTitle="Your inbox is empty"
      emptyDescription="Use Quick capture in the header or the New item button to add something."
      baseFilters={{ statuses: ['inbox'], sort: 'created_at', order: 'desc' }}
    />
  )
}
