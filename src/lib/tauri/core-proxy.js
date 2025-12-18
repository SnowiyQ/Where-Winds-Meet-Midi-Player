import { invoke as originalInvoke } from '@tauri-apps/api/core'
import { logUiAction } from '../utils/uiActionLogger.js'

const skippedCommands = new Set()
const isTestEnv = (typeof process !== 'undefined' && process.env.NODE_ENV === 'test') || import.meta.env?.VITEST === true
const isTauri = typeof window !== 'undefined' && Boolean(window.__TAURI__)

function logSkip(command) {
  if (!command || isTestEnv || skippedCommands.has(command)) {
    return
  }
  skippedCommands.add(command)
  console.warn(`Skipping Tauri command outside of Tauri: ${command}`)
  logUiAction('tauri.invoke', 'warn', { command, reason: 'not_running_in_tauri' })
}

function summarizeArg(arg) {
  if (arg === null) return 'null'
  if (arg === undefined) return 'undefined'
  if (Array.isArray(arg)) return `Array(${arg.length})`
  if (typeof arg === 'object') {
    const keys = Object.keys(arg)
    return `${arg.constructor?.name || 'Object'}(${keys.length})`
  }
  if (typeof arg === 'function') return `Function(${arg.name || 'anonymous'})`
  return arg
}

export const invoke = (...args) => {
  const [command, ...rest] = args
  if (!isTauri) {
    logSkip(command)
    return Promise.resolve(null)
  }

  const context = {
    command,
    params: rest.map((param) => summarizeArg(param))
  }

  logUiAction('tauri.invoke', 'started', context)
  return originalInvoke(...args)
    .then((result) => {
      logUiAction('tauri.invoke', 'completed', context)
      return result
    })
    .catch((error) => {
      logUiAction('tauri.invoke', 'error', { ...context, error: error?.message || error })
      throw error
    })
}
