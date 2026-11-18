import { describe, expect, it } from 'vitest'

import { buildItemsQuery } from './items'

function parse(query: string): URLSearchParams {
  return new URLSearchParams(query.startsWith('?') ? query.slice(1) : query)
}

describe('buildItemsQuery', () => {
  it('is empty when there are no filters', () => {
    expect(buildItemsQuery({})).toBe('')
  })

  it('maps camelCase filters to the API vocabulary', () => {
    const params = parse(
      buildItemsQuery({
        q: '  milk ',
        type: 'task',
        statuses: ['inbox', 'pending'],
        priority: 'high',
        category: ' errands ',
        tag: 'home',
        dueFrom: '2026-11-18T00:00:00Z',
        dueTo: '2026-11-19T00:00:00Z',
        sort: 'due_at',
        order: 'asc',
        page: 2,
        pageSize: 50,
      }),
    )

    expect(params.get('q')).toBe('milk')
    expect(params.get('type')).toBe('task')
    expect(params.get('status')).toBe('inbox,pending')
    expect(params.get('priority')).toBe('high')
    expect(params.get('category')).toBe('errands')
    expect(params.get('tag')).toBe('home')
    expect(params.get('due_from')).toBe('2026-11-18T00:00:00Z')
    expect(params.get('due_to')).toBe('2026-11-19T00:00:00Z')
    expect(params.get('sort')).toBe('due_at')
    expect(params.get('order')).toBe('asc')
    expect(params.get('page')).toBe('2')
    expect(params.get('page_size')).toBe('50')
  })

  it('drops blank values instead of sending empty parameters', () => {
    const params = parse(
      buildItemsQuery({ q: '   ', category: '', statuses: [] }),
    )

    expect([...params.keys()]).toEqual([])
  })
})
