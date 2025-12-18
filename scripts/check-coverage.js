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

const sections = raw
  .split('end_of_record')
  .map((section) => section.trim())
  .filter(Boolean)

let totalLines = 0
let coveredLines = 0
for (const section of sections) {
  const lfMatch = section.match(/LF:(\d+)/)
  const lhMatch = section.match(/LH:(\d+)/)
  if (lfMatch && lhMatch) {
    totalLines += Number(lfMatch[1])
    coveredLines += Number(lhMatch[1])
  }
}

const coveragePercent = totalLines === 0 ? 0 : Math.round((coveredLines / totalLines) * 100)
const summary = `line coverage ${coveragePercent}% (${coveredLines}/${totalLines})`
console.log(`Coverage check: ${summary}`)
if (coveragePercent < threshold) {
  console.log(`::warning::${summary} is below the ${threshold}% target. See coverage/lcov-report/index.html for details.`)
} else {
  console.log(`Coverage meets the ${threshold}% threshold.`)
}