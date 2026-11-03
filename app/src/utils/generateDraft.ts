export const hasControl = (value: string) =>
  [...value].some((character) => {
    const code = character.codePointAt(0) ?? 0
    return code < 32 || (code >= 127 && code <= 159)
  })

export function validText(value: string, required = false) {
  const trimmed = value.trim()
  return (!required || trimmed.length > 0) && [...trimmed].length <= 200 && !hasControl(trimmed)
}

export function validNumber(value: number | null, min: number, max: number) {
  return value === null || (Number.isFinite(value) && value >= min && value <= max)
}

export function validTags(tags: string[]) {
  const values = tags.map((tag) => tag.trim())
  return (
    values.length <= 16 &&
    values.every(
      (tag) =>
        [...tag].length >= 1 &&
        [...tag].length <= 64 &&
        ![...tag].some((character) => /\s/u.test(character) || ':=[]'.includes(character)) &&
        !hasControl(tag),
    ) &&
    new Set(values.map((tag) => tag.toLocaleLowerCase())).size === values.length
  )
}

export function publicTags(tags: string[]) {
  const mandatory = ['Beatforge', 'AI', 'mapperatorinator']
  const reserved = new Set(mandatory.map((tag) => tag.toLocaleLowerCase()))
  const userTags = tags
    .map((tag) => tag.trim())
    .filter((tag) => !reserved.has(tag.toLocaleLowerCase()))
  return [...userTags, ...mandatory].join(' ')
}
