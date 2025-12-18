import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'

const threshold = Number(process.env.COVERAGE_THRESHOLD ?? 80)
const reportPath = resolve('coverage', 'lcov.info')
let raw
try {
  raw = readFileSync(reportPath, 'utf8')
} catch (error) {
  console.log(`::warning::Coverage report not found at ${reportPath}. Run npm run test:coverage first.`)
  process.exit(0)
}

const trackedPaths = ['src/lib/version.js', 'src/lib/utils']
const normalize = (value) => value.replace(/\\/g, '/')

const sections = raw
  .split('end_of_record')
  .map((section) => section.trim())
  .filter(Boolean)

const categorize = (section) => {
  const match = section.match(/^SF:(.+)$/m)
  if (!match) return null
  const filePath = normalize(match[1])
  const isTracked = trackedPaths.some((target) => filePath === target || filePath.startsWith(`${target}/`))
  return { section, filePath, isTracked }
}

const trackedSections = []
const otherSections = []
for (const section of sections) {
  const entry = categorize(section)
  if (!entry) continue
  if (entry.isTracked) {
    trackedSections.push(entry.section)
  } else {
    otherSections.push(entry.section)
  }
}

const reduceCoverage = (items) => {
  let lines = 0
  let hits = 0
  for (const section of items) {
    const lfMatch = section.match(/LF:(\d+)/)
    const lhMatch = section.match(/LH:(\d+)/)
    if (lfMatch && lhMatch) {
      lines += Number(lfMatch[1])
      hits += Number(lhMatch[1])
    }
  }
  return { lines, hits }
}

const trackedStats = reduceCoverage(trackedSections)
if (trackedStats.lines === 0) {
  console.log('Coverage check: no tracked files were found in coverage/lcov.info')
  process.exit(0)
}

const coveragePercent = Math.round((trackedStats.hits / trackedStats.lines) * 100)
const summary = `line coverage ${coveragePercent}% (${trackedStats.hits}/${trackedStats.lines})`
console.log(`Coverage check (tracked libs): ${summary}`)
if (coveragePercent < threshold) {
  console.log(`::warning::${summary} is below the ${threshold}% target. See coverage/lcov-report/index.html for details.`)
} else {
  console.log(`Coverage meets the ${threshold}% threshold.`)
}

if (otherSections.length > 0) {
  const otherStats = reduceCoverage(otherSections)
  const otherPercent = otherStats.lines === 0 ? 0 : Math.round((otherStats.hits / otherStats.lines) * 100)
  console.log(`Additional files (e.g., Svelte components) are included in coverage/lcov-report (line coverage ${otherPercent}% across ${otherSections.length} sections).`)
}