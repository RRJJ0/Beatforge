import { describe, expect, it } from 'vitest'
import type { DifficultyRequest } from '@/types'
import { capabilitiesForModel, supportedDifficulty } from '@/utils/modelCapabilities'

const draft: DifficultyRequest = {
  id: 'difficulty-1',
  name: 'Normal',
  difficulty: 4.5,
  seed: 7,
  descriptors: ['streams'],
  negativeDescriptors: ['jumps'],
  hitsounded: true,
}

describe('model request capabilities', () => {
  it('omits unsupported V30 conditioning without changing the draft', () => {
    const capabilities = capabilitiesForModel('mapperatorinator-v30-standard')!
    expect(supportedDifficulty(draft, capabilities)).toMatchObject({
      difficulty: 0,
      descriptors: [],
      negativeDescriptors: [],
      hitsounded: false,
    })
    expect(draft.difficulty).toBe(4.5)
    expect(draft.descriptors).toEqual(['streams'])
  })

  it('preserves V31 descriptors and V32-mini target difficulty', () => {
    const v31 = supportedDifficulty(draft, capabilitiesForModel('mapperatorinator-v31-standard')!)
    expect(v31.difficulty).toBe(0)
    expect(v31.descriptors).toEqual(['streams'])
    expect(v31.hitsounded).toBe(true)
    const mini = supportedDifficulty(
      draft,
      capabilitiesForModel('mapperatorinator-v32-mini-standard')!,
    )
    expect(mini.difficulty).toBe(4.5)
    expect(capabilitiesForModel('unknown')).toBeUndefined()
  })
})
