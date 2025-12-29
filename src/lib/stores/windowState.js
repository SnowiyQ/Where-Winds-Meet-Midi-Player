import { writable, get } from 'svelte/store'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { PhysicalPosition, PhysicalSize } from '@tauri-apps/api/dpi'
import { invoke } from '../tauri/core-proxy.js'

export const relativeWindowBounds = writable(null)

export async function rememberWindowBoundsRelativeToGame() {
  try {
    const appWindow = getCurrentWindow()
    const [pos, size, gameBounds] = await Promise.all([
      appWindow.innerPosition(),
      appWindow.innerSize(),
      invoke('get_game_window_bounds')
    ])

    const data = {
      absolute: { x: pos.x, y: pos.y, width: size.width, height: size.height },
      relative: null
    }

    if (gameBounds && gameBounds.width && gameBounds.height) {
      data.relative = {
        x: (pos.x - gameBounds.x) / gameBounds.width,
        y: (pos.y - gameBounds.y) / gameBounds.height,
        width: size.width / gameBounds.width,
        height: size.height / gameBounds.height
      }
    }

    relativeWindowBounds.set(data)
    return data
  } catch (error) {
    console.error('Failed to capture window bounds:', error)
    return null
  }
}

export async function restoreWindowBounds(defaultSize = { width: 1180, height: 620 }) {
  const saved = get(relativeWindowBounds)
  const appWindow = getCurrentWindow()

  const fallback = saved?.absolute || {
    x: null,
    y: null,
    width: defaultSize.width,
    height: defaultSize.height
  }

  let target = { ...fallback }

  try {
    const gameBounds = await invoke('get_game_window_bounds')
    if (gameBounds && saved?.relative) {
      target = {
        x: Math.round(gameBounds.x + saved.relative.x * gameBounds.width),
        y: Math.round(gameBounds.y + saved.relative.y * gameBounds.height),
        width: Math.max(64, Math.round(saved.relative.width * gameBounds.width)),
        height: Math.max(64, Math.round(saved.relative.height * gameBounds.height))
      }
    }
  } catch (error) {
    console.error('Failed to compute relative bounds:', error)
  }

  try {
    if (target.width && target.height) {
      await appWindow.setSize(new PhysicalSize(target.width, target.height))
    }
    if (target.x !== null && target.y !== null) {
      await appWindow.setPosition(new PhysicalPosition(target.x, target.y))
    }
  } catch (error) {
    console.error('Failed to restore window bounds:', error)
  }

  return target
}
