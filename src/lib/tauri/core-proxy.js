import { invoke as originalInvoke } from '@tauri-apps/api/core'

const skippedCommands = new Set()
const isTestEnv = (typeof process !== 'undefined' && process.env.NODE_ENV === 'test') || import.meta.env?.VITEST === true
const isTauri = typeof window !== 'undefined' && Boolean(window.__TAURI__)

function logSkip(command) {
  if (!command || isTestEnv || skippedCommands.has(command)) {
    return
  }
  skippedCommands.add(command)
  console.warn(`Skipping Tauri command outside of Tauri: ${command}`)
}

export const invoke = (...args) => {
  const [command] = args
  if (!isTauri) {
    logSkip(command)
    return Promise.resolve(null)
  }
  return originalInvoke(...args)
}
