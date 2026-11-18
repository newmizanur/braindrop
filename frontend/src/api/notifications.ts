import { useMutation } from '@tanstack/react-query'

import type { NotificationChannel } from '../bindings/NotificationChannel'
import type { TestNotificationParams } from '../bindings/TestNotificationParams'
import type { TestNotificationResponse } from '../bindings/TestNotificationResponse'
import { post } from './client'

/**
 * Ask the server to send one test message through a configured channel.
 * Credentials never leave the server; only the channel name is sent.
 */
export function useSendTestNotification() {
  return useMutation({
    mutationFn: (channel: NotificationChannel) =>
      post<TestNotificationResponse>(
        '/api/notifications/test',
        { channel } satisfies TestNotificationParams,
      ),
  })
}
