import { describe, expect, it } from 'vitest'
import { initials } from './initials'

describe('initials', () => {
  it('uses the first and last name', () => {
    expect(initials('Ada Lovelace')).toBe('AL')
  })

  it('uses two letters for a single name', () => {
    expect(initials('Ada')).toBe('AD')
  })

  it('handles extra whitespace and empty names', () => {
    expect(initials('  Grace   Brewster   Hopper ')).toBe('GH')
    expect(initials('   ')).toBe('?')
  })
})
