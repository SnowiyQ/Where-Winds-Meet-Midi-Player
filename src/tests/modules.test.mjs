import { describe, expect, it } from 'vitest'
import * as bandStore from '../lib/stores/band.js'
import * as keyboardStore from '../lib/stores/keyboard.js'
import * as libraryStore from '../lib/stores/library.js'
import * as playerStore from '../lib/stores/player.js'
import * as i18n from '../lib/i18n/index.js'
import postcssConfig from '../../postcss.config.js'
import tailwindConfig from '../../tailwind.config.js'

const stores = [
  bandStore,
  keyboardStore,
  libraryStore,
  playerStore,
]

describe('store modules initialize', () => {
  it('exports helper objects without throwing', () => {
    for (const store of stores) {
      expect(store).toBeTruthy()
    }
  })
})

describe('i18n helpers', () => {
  it('exports waitLocale and languages', () => {
    expect(i18n.waitLocale).toBeInstanceOf(Function)
    expect(Array.isArray(i18n.languages)).toBe(true)
  })
})

describe('config files', () => {
  it('load PostCSS configuration', () => {
    expect(postcssConfig).toBeTruthy()
  })

  it('load Tailwind configuration', () => {
    expect(tailwindConfig.theme).toBeTruthy()
  })
})