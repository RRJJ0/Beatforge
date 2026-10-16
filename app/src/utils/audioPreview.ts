export function downsamplePeaks(samples: Float32Array, maxPoints: number): number[] {
  if (!samples.length || maxPoints <= 0) return []
  const pointCount = Math.min(samples.length, Math.floor(maxPoints))
  const stride = samples.length / pointCount
  const peaks = Array.from({ length: pointCount }, () => 0)
  for (let point = 0; point < pointCount; point += 1) {
    const start = Math.floor(point * stride)
    const end = Math.max(start + 1, Math.floor((point + 1) * stride))
    let peak = 0
    for (let index = start; index < end && index < samples.length; index += 1) {
      peak = Math.max(peak, Math.abs(samples[index] ?? 0))
    }
    peaks[point] = peak
  }
  return peaks
}

export function formatPreviewTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return '0:00'
  const rounded = Math.floor(seconds)
  return `${Math.floor(rounded / 60)}:${String(rounded % 60).padStart(2, '0')}`
}
