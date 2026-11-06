import { beforeEach, describe, expect, it, vi } from 'vitest'

describe('interface locale', () => {
  beforeEach(() => {
    const values = new Map<string, string>()
    vi.stubGlobal('localStorage', {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
      clear: () => values.clear(),
    })
  })

  it('switches translations and updates the document language', async () => {
    const { useI18n } = await import('@/i18n')
    const { setLocale, t } = useI18n()

    setLocale('zh-TW')
    expect(t('nav.generate')).toBe('生成')
    expect(t('preview.objects', { count: 12 })).toBe('12 個物件')
    expect(document.documentElement.lang).toBe('zh-TW')
    expect(localStorage.getItem('beatforge:locale')).toBe('zh-TW')

    setLocale('en')
    expect(t('nav.generate')).toBe('Generate')
  })
})
