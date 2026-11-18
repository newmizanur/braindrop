import dayjs from 'dayjs'
import timezonePlugin from 'dayjs/plugin/timezone'
import utcPlugin from 'dayjs/plugin/utc'

import type { ItemPriority } from '../bindings/ItemPriority'
import type { ItemResponse } from '../bindings/ItemResponse'
import type { ItemStatus } from '../bindings/ItemStatus'
import type { ItemType } from '../bindings/ItemType'

dayjs.extend(utcPlugin)
dayjs.extend(timezonePlugin)

export const ITEM_TYPES: ItemType[] = ['task', 'note', 'idea', 'reminder']
export const ITEM_STATUSES: ItemStatus[] = [
  'inbox',
  'pending',
  'completed',
  'archived',
]
export const ITEM_PRIORITIES: ItemPriority[] = [
  'low',
  'normal',
  'high',
  'urgent',
]

const typeLabels: Record<ItemType, string> = {
  task: 'Task',
  note: 'Note',
  idea: 'Idea',
  reminder: 'Reminder',
}

const priorityLabels: Record<ItemPriority, string> = {
  low: 'Low',
  normal: 'Normal',
  high: 'High',
  urgent: 'Urgent',
}

const statusLabels: Record<ItemStatus, string> = {
  inbox: 'Inbox',
  pending: 'Pending',
  completed: 'Completed',
  archived: 'Archived',
}

export function itemTypeLabel(type: ItemType): string {
  return typeLabels[type]
}

export function priorityLabel(priority: ItemPriority): string {
  return priorityLabels[priority]
}

export function statusLabel(status: ItemStatus): string {
  return statusLabels[status]
}

export function itemTypeColor(type: ItemType): string {
  switch (type) {
    case 'task':
      return 'indigo'
    case 'note':
      return 'grape'
    case 'idea':
      return 'teal'
    case 'reminder':
      return 'orange'
  }
}

export function priorityColor(priority: ItemPriority): string {
  switch (priority) {
    case 'low':
      return 'gray'
    case 'normal':
      return 'blue'
    case 'high':
      return 'orange'
    case 'urgent':
      return 'red'
  }
}

export function statusColor(status: ItemStatus): string {
  switch (status) {
    case 'inbox':
      return 'indigo'
    case 'pending':
      return 'blue'
    case 'completed':
      return 'teal'
    case 'archived':
      return 'gray'
  }
}

export function isOpen(item: Pick<ItemResponse, 'status'>): boolean {
  return item.status === 'inbox' || item.status === 'pending'
}

export function isOverdue(
  item: Pick<ItemResponse, 'status' | 'due_at'>,
  now: Date = new Date(),
): boolean {
  if (!item.due_at || !isOpen(item)) {
    return false
  }
  return dayjs(item.due_at).isBefore(now)
}

export function isDueToday(
  item: Pick<ItemResponse, 'due_at'>,
  timezone: string,
  now: Date = new Date(),
): boolean {
  if (!item.due_at) {
    return false
  }
  return dayjs(item.due_at)
    .tz(timezone)
    .isSame(dayjs(now).tz(timezone), 'day')
}

/** ISO-8601 instant for the start of the current day in `timezone`. */
export function startOfToday(timezone: string, now: Date = new Date()): string {
  return dayjs(now).tz(timezone).startOf('day').toISOString()
}

/** Half-open `[start, end)` bounds for today in `timezone`. */
export function todayBounds(
  timezone: string,
  now: Date = new Date(),
): { start: string; end: string } {
  const start = dayjs(now).tz(timezone).startOf('day')
  return {
    start: start.toISOString(),
    end: start.add(1, 'day').toISOString(),
  }
}

/** Half-open bounds for the next seven days, starting tomorrow. */
export function upcomingBounds(
  timezone: string,
  now: Date = new Date(),
): { start: string; end: string } {
  const start = dayjs(now).tz(timezone).startOf('day').add(1, 'day')
  return {
    start: start.toISOString(),
    end: start.add(7, 'day').toISOString(),
  }
}

export function formatDateTime(iso: string): string {
  return dayjs(iso).format('MMM D, YYYY HH:mm')
}

export function formatDueDate(iso: string): string {
  const due = dayjs(iso)
  if (due.isSame(dayjs(), 'day')) {
    return `Today ${due.format('HH:mm')}`
  }
  if (due.isSame(dayjs().add(1, 'day'), 'day')) {
    return `Tomorrow ${due.format('HH:mm')}`
  }
  return due.format('MMM D, HH:mm')
}

/** Convert a Mantine date-picker value to a UTC ISO string. */
export function toIso(
  value: string | Date | null | undefined,
): string | null {
  if (!value) {
    return null
  }
  const date = value instanceof Date ? value : new Date(value)
  if (Number.isNaN(date.getTime())) {
    return null
  }
  return date.toISOString()
}

/** Parse a comma-separated tag input into a clean, unique list. */
export function parseTags(value: string): string[] {
  const seen = new Set<string>()
  return value
    .split(',')
    .map((tag) => tag.trim())
    .filter((tag) => tag.length > 0)
    .filter((tag) => {
      const key = tag.toLowerCase()
      if (seen.has(key)) {
        return false
      }
      seen.add(key)
      return true
    })
}

export function formatTags(tags: string[]): string {
  return tags.join(', ')
}
