import { describe, expect, it } from 'vitest'
import { publicTags, validNumber, validTags, validText } from '../generateDraft'

describe('0.2.0 generate draft fields', () => {
  it('keeps optional text empty but rejects controls and oversized values', () => {
    expect(validText('')).toBe(true)
    expect(validText(' ', true)).toBe(false)
    expect(validText('歌曲', true)).toBe(true)
    expect(validText('a\nsecond line')).toBe(false)
    expect(validText('x'.repeat(201))).toBe(false)
  })

  it('allows omitted numeric settings and rejects non-finite or out-of-range values', () => {
    expect(validNumber(null, 0.1, 10)).toBe(true)
    expect(validNumber(0.1, 0.1, 10)).toBe(true)
    expect(validNumber(0, 0.1, 10)).toBe(false)
    expect(validNumber(Number.NaN, 0, 10)).toBe(false)
    expect(validNumber(Number.POSITIVE_INFINITY, 0, 10)).toBe(false)
  })

  it('deduplicates mandatory disclosure tags and rejects unsafe public tags', () => {
    expect(validTags(['electronic', 'j-pop'])).toBe(true)
    expect(validTags(['Electronic', 'electronic'])).toBe(false)
    expect(validTags(['two words'])).toBe(false)
    expect(validTags(['seed=42'])).toBe(false)
    expect(validTags([''])).toBe(false)
    expect(publicTags(['electronic', 'AI'])).toBe('electronic Beatforge AI mapperatorinator')
  })
})
