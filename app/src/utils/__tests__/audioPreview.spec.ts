import { describe, expect, it } from 'vitest'
import { downsamplePeaks, formatPreviewTime } from '@/utils/audioPreview'

describe('audio preview helpers', () => {
  it('limits waveform points and preserves peaks', () => {
    const result = downsamplePeaks(new Float32Array([0, -0.5, 0.2, 1, -0.25, 0.1]), 3)
    expect(result).toEqual([0.5, 1, 0.25])
  })

  it('handles empty and invalid time values', () => {
    expect(downsamplePeaks(new Float32Array(), 320)).toEqual([])
    expect(formatPreviewTime(Number.NaN)).toBe('0:00')
    expect(formatPreviewTime(125.8)).toBe('2:05')
  })
})
