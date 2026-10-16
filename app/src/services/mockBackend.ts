export interface MockTask {
  cancel: () => void
}

export function simulateProgress(
  onProgress: (percent: number) => void,
  onComplete: () => void,
  intervalMs = 420,
): MockTask {
  let percent = 0
  const timer = window.setInterval(() => {
    percent = Math.min(100, percent + Math.ceil(Math.random() * 14))
    onProgress(percent)
    if (percent >= 100) {
      window.clearInterval(timer)
      onComplete()
    }
  }, intervalMs)
  return { cancel: () => window.clearInterval(timer) }
}

export async function mockEnvironmentCheck() {
  await new Promise((resolve) => window.setTimeout(resolve, 650))
  return {
    python: '3.11.9',
    mapperatorinator: 'ready',
    pytorch: '2.5',
    cuda: '12.4',
    gpu: 'RTX 4070',
  }
}
