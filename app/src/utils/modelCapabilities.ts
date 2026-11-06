import type { DifficultyRequest } from '@/types'

export interface ModelCapabilities {
  targetDifficulty: boolean
  descriptors: boolean
  hitsounded: boolean
  referenceTiming: boolean
}

const full: ModelCapabilities = {
  targetDifficulty: true,
  descriptors: true,
  hitsounded: true,
  referenceTiming: true,
}

const models: Record<string, ModelCapabilities> = {
  'mapperatorinator-v30-standard': {
    targetDifficulty: false,
    descriptors: false,
    hitsounded: false,
    referenceTiming: false,
  },
  'mapperatorinator-v31-standard': {
    targetDifficulty: false,
    descriptors: true,
    hitsounded: true,
    referenceTiming: false,
  },
  'mapperatorinator-v32-mini-standard': {
    ...full,
    referenceTiming: false,
  },
  'mapperatorinator-v32-standard': full,
}

export function capabilitiesForModel(modelId: string): ModelCapabilities | undefined {
  return models[modelId]
}

export function supportedDifficulty(
  difficulty: DifficultyRequest,
  capabilities: ModelCapabilities,
): DifficultyRequest {
  return {
    ...difficulty,
    difficulty: capabilities.targetDifficulty ? difficulty.difficulty : 0,
    descriptors: capabilities.descriptors ? difficulty.descriptors : [],
    negativeDescriptors: capabilities.descriptors ? difficulty.negativeDescriptors : [],
    hitsounded: capabilities.hitsounded && difficulty.hitsounded,
  }
}
