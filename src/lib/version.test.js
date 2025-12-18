import { describe, expect, it } from 'vitest'
import { APP_FLAVOR, APP_VERSION } from './version.js'

describe('version metadata', () => {
  it('uses a semantic version string', () => {
    expect(APP_VERSION).toMatch(/^\d+\.\d+\.\d+(?:-[^\s]+)?$/)
  })

  it('always exposes a flavor marker', () => {
    expect(APP_FLAVOR).toBeTruthy()
  })
})
