import { describe, expect, it } from 'vitest'

import type { ItemResponse } from '../bindings/ItemResponse'
import {
  formatDueDate,
  isDueToday,
  isOverdue,
  parseTags,
  todayBounds,
  toIso,
  upcomingBounds,
} from './items'

function item(overrides: Partial<ItemResponse> = {}): ItemResponse {
  return {
    pid: '11111111-1111-1111-1111-111111111111',
    type: 'task',
    title: 'Example',
    content: null,
    category: null,
    tags: [],
    status: 'inbox',
    priority: 'normal',
    due_at: null,
    remind_at: null,
    completed_at: null,
    archived_at: null,
    created_at: '2026-11-18T10:00:00Z',
    updated_at: '2026-11-18T10:00:00Z',
    ...overrides,
  }
}

describe('parseTags', () => {
  it('trims, drops empties and de-duplicates case-insensitively', () => {
    expect(parseTags(' Work, home ,, WORK, deep ')).toEqual(['Work', 'home', 'deep'])
  })

  it('returns an empty list for blank input', () => {
    expect(parseTags('   ,  ')).toEqual([])
  })
})

describe('toIso', () => {
  it('converts a Date to an ISO string', () => {
    expect(toIso(new Date('2026-12-01T09:00:00Z'))).toBe(
      '2026-12-01T09:00:00.000Z',
    )
  })

  it('returns null for missing or invalid values', () => {
    expect(toIso(null)).toBeNull()
    expect(toIso(undefined)).toBeNull()
    expect(toIso('not-a-date')).toBeNull()
  })
})

describe('todayBounds', () => {
  it('returns a 24 hour, half-open window', () => {
    const now = new Date('2026-11-18T12:00:00Z')
    const { start, end } = todayBounds('UTC', now)
    expect(start).toBe('2026-11-18T00:00:00.000Z')
    expect(end).toBe('2026-11-19T00:00:00.000Z')
    expect(new Date(end).getTime() - new Date(start).getTime()).toBe(
      24 * 60 * 60 * 1000,
    )
  })

  it('follows the configured timezone', () => {
    const now = new Date('2026-11-18T12:00:00Z')
    const { start } = todayBounds('Europe/Berlin', now)
    // Berlin is UTC+1 in November.
    expect(start).toBe('2026-11-17T23:00:00.000Z')
  })
})

describe('upcomingBounds', () => {
  it('starts tomorrow and covers seven days', () => {
    const now = new Date('2026-11-18T12:00:00Z')
    const { start, end } = upcomingBounds('UTC', now)
    expect(start).toBe('2026-11-19T00:00:00.000Z')
    expect(end).toBe('2026-11-26T00:00:00.000Z')
  })
})

describe('isDueToday', () => {
  it('compares calendar days in the given timezone', () => {
    // 23:30 UTC on the 18th is already the 19th in Tokyo.
    const due = '2026-11-18T23:30:00Z'
    const now = new Date('2026-11-18T12:00:00Z')
    expect(isDueToday({ due_at: due }, 'UTC', now)).toBe(true)
    expect(isDueToday({ due_at: due }, 'Asia/Tokyo', now)).toBe(false)
  })

  it('is false without a due date', () => {
    expect(isDueToday({ due_at: null }, 'UTC')).toBe(false)
  })
})

describe('isOverdue', () => {
  const now = new Date('2026-11-18T12:00:00Z')

  it('only considers open items with a past due date', () => {
    expect(
      isOverdue({ status: 'inbox', due_at: '2026-11-18T09:00:00Z' }, now),
    ).toBe(true)
    expect(
      isOverdue({ status: 'completed', due_at: '2026-11-18T09:00:00Z' }, now),
    ).toBe(false)
    expect(
      isOverdue({ status: 'inbox', due_at: '2026-11-19T09:00:00Z' }, now),
    ).toBe(false)
    expect(isOverdue({ status: 'inbox', due_at: null }, now)).toBe(false)
  })
})

describe('formatDueDate', () => {
  it('formats a date far from today with the month, day and time', () => {
    expect(formatDueDate('2020-12-01T09:30:00Z')).toMatch(
      /^[A-Z][a-z]{2} \d{1,2}, \d{2}:\d{2}$/,
    )
  })
})

describe('item defaults', () => {
  it('keeps the shape used by the API', () => {
    expect(item().status).toBe('inbox')
  })
})
