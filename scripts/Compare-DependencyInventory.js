const { readFileSync } = require('node:fs')

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical)
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonical(value[key])]),
    )
  }
  return value
}

function inventory(path) {
  const value = JSON.parse(readFileSync(path, 'utf8'))
  for (const group of ['rust', 'npm', 'python', 'managedComponents']) {
    value[group].sort((left, right) => {
      const key = (record) => [
        record.name,
        record.version ?? '',
        record.source ?? '',
        JSON.stringify(canonical(record)),
      ].join('\0')
      const a = key(left)
      const b = key(right)
      return a < b ? -1 : a > b ? 1 : 0
    })
  }
  return canonical(value)
}

function firstDifference(left, right, path = '$') {
  if (Object.is(left, right)) return null
  if (left === null || right === null || typeof left !== 'object' || typeof right !== 'object') {
    return path
  }
  const leftKeys = Object.keys(left)
  const rightKeys = Object.keys(right)
  if (JSON.stringify(leftKeys) !== JSON.stringify(rightKeys)) return path
  for (const key of leftKeys) {
    const difference = firstDifference(left[key], right[key], `${path}.${key}`)
    if (difference !== null) return difference
  }
  return null
}

const difference = firstDifference(inventory(process.argv[2]), inventory(process.argv[3]))
if (difference !== null) {
  process.stderr.write(`Inventory differs at ${difference}\n`)
  if (difference.startsWith('$.rust.')) {
    const index = Number(difference.split('.')[2])
    const left = inventory(process.argv[2]).rust[index]
    const right = inventory(process.argv[3]).rust[index]
    process.stderr.write(`Records: ${left.name}@${left.version} / ${right.name}@${right.version}\n`)
  }
  process.exitCode = 1
}
