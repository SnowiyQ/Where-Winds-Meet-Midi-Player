import { beforeEach, describe, expect, it, vi } from 'vitest'
import enLocale from '../lib/i18n/locales/en.json'

vi.mock('svelte-i18n', () => {
  const register = vi.fn()
  const init = vi.fn()
  const locale = { set: vi.fn() }
  const waitLocale = vi.fn(() => Promise.resolve())

  return {
    register,
    init,
    locale,
    waitLocale,
    t: vi.fn(),
  }
})

vi.mock('../lib/tauri/core-proxy.js', () => ({
  invoke: vi.fn(() => Promise.resolve(null)),
}))

import { register, init, locale, waitLocale } from 'svelte-i18n'
import { invoke } from '../lib/tauri/core-proxy.js'

import {
  deepMerge,
  hasNewKeys,
  loadLocaleWithUserOverrides,
  initUserLocales,
  openLocalesFolder,
  getLocalesPath,
  reloadCurrentLocale,
} from '../lib/i18n/index.js'

const registerMock = vi.mocked(register)
const initMock = vi.mocked(init)
const localeSetMock = vi.mocked(locale.set)
const waitLocaleMock = vi.mocked(waitLocale)
const invokeMock = vi.mocked(invoke)

describe('i18n helpers', () => {
  beforeEach(() => {
    invokeMock.mockReset().mockResolvedValue(null)
    registerMock.mockReset()
    initMock.mockReset()
    localeSetMock.mockReset()
    waitLocaleMock.mockReset().mockResolvedValue()
    localStorage.clear()
  })

  it('deepMerge merges nested objects and overrides values', () => {
    const target = { a: { b: 1 }, c: 2 }
    const source = { a: { d: 3 }, e: 4 }
    const merged = deepMerge(target, source)
    expect(merged).toMatchObject({ a: { b: 1, d: 3 }, c: 2, e: 4 })
  })

  it('hasNewKeys detects when bundled locale adds keys', () => {
    const bundled = { header: { title: 'App' }, footer: { text: 'ok' } }
    const user = { header: { title: 'App' } }
    expect(hasNewKeys(bundled, user)).toBe(true)
    expect(hasNewKeys(bundled, bundled)).toBe(false)
  })

  it('loadLocaleWithUserOverrides merges missing keys and saves', async () => {
    const userLocale = { header: { title: 'Custom header' } }
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'get_user_locale') return userLocale
      if (cmd === 'save_user_locale') return null
      return null
    })

    const merged = await loadLocaleWithUserOverrides('en')
    expect(invokeMock).toHaveBeenCalledWith('get_user_locale', { lang: 'en' })
    expect(invokeMock).toHaveBeenCalledWith(
      'save_user_locale',
      expect.objectContaining({ lang: 'en' })
    )
    expect(merged.header.title).toBe('Custom header')
    expect(merged.nav).toBeDefined()
  })

  it('returns user locale when it already contains all keys', async () => {
    const userCopy = JSON.parse(JSON.stringify(enLocale))
    invokeMock.mockResolvedValue(userCopy)

    const result = await loadLocaleWithUserOverrides('en')
    expect(result).toBe(userCopy)
    expect(invokeMock).toHaveBeenCalledWith('get_user_locale', { lang: 'en' })
    expect(invokeMock).not.toHaveBeenCalledWith('save_user_locale', expect.anything())
  })

  it('falls back to bundled locale when user locale fails to load', async () => {
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'get_user_locale') throw new Error('failed')
      return null
    })

    const fallback = await loadLocaleWithUserOverrides('en')
    expect(fallback.nav).toBeDefined()
    expect(invokeMock).toHaveBeenCalledWith('get_user_locale', { lang: 'en' })
  })

  it('initUserLocales calls the backend with defaults', async () => {
    await initUserLocales()
    expect(invokeMock).toHaveBeenCalledWith(
      'init_user_locales',
      expect.objectContaining({ defaultLocales: expect.any(Object) })
    )
  })

  it('openLocalesFolder triggers the invoke helper', async () => {
    await openLocalesFolder()
    expect(invokeMock).toHaveBeenCalledWith('open_locales_folder')
  })

  it('getLocalesPath returns the current path', async () => {
    invokeMock.mockResolvedValue('/tmp/locales')
    const path = await getLocalesPath()
    expect(path).toBe('/tmp/locales')
  })

  it('reloadCurrentLocale resets locale state and waits twice', async () => {
    localStorage.setItem('wwm-language', 'zh')
    await reloadCurrentLocale()
    expect(localeSetMock).toHaveBeenNthCalledWith(1, null)
    expect(localeSetMock).toHaveBeenNthCalledWith(2, 'zh')
    expect(waitLocaleMock).toHaveBeenCalledTimes(2)
  })
})
