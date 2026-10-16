// Keep Cargo metadata parsing compatible with Windows PowerShell 5.1. Node
// preserves UTF-8 stdout and accepts unrelated metadata keys differing by case.
const { execFileSync } = require('node:child_process')

const raw = execFileSync('cargo.exe', [
  'metadata',
  '--offline',
  '--locked',
  '--format-version',
  '1',
  '--manifest-path',
  process.argv[2],
], { maxBuffer: 32 * 1024 * 1024 })
const packages = JSON.parse(raw.toString('utf8')).packages.map(
  ({ name, version, license, authors, repository, source }) => ({
    name,
    version,
    license,
    authors,
    repository,
    source,
  }),
)
const asciiJson = JSON.stringify({ packages }).replace(/[^\x00-\x7F]/g, (char) =>
  `\\u${char.charCodeAt(0).toString(16).padStart(4, '0')}`,
)
process.stdout.write(asciiJson)
