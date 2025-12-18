import path from 'node:path'
import { execFile } from 'node:child_process'
import { promisify } from 'node:util'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { writeFile, mkdir } from 'node:fs/promises'
import { rmSync } from 'node:fs'
import crypto from 'node:crypto'
import { describe, expect, it } from 'vitest'

const execFileAsync = promisify(execFile)
const scriptPath = path.resolve('scripts/buildComponent.mjs')
const testsDir = path.dirname(fileURLToPath(import.meta.url))
const smokeBundlesDir = path.resolve(testsDir, '..', '..', '.temp', 'components-smoke')
const ensureBundlesDir = mkdir(smokeBundlesDir, { recursive: true })
const tempFiles = new Set()
const tauriStub = {
  invoke: async () => ({}),
  event: {
    listen: async () => ({
      unlisten: async () => null,
    }),
  },
  emit: async () => ({}),
}
globalThis.__TAURI__ = globalThis.__TAURI__ ?? tauriStub
if (typeof window !== 'undefined') {
  window.__TAURI__ = globalThis.__TAURI__
}

const components = [
  { label: 'src/App.svelte', path: '../App.svelte' },
  { label: 'src/lib/components/BandMode.svelte', path: '../lib/components/BandMode.svelte' },
  { label: 'src/lib/components/FavoritesView.svelte', path: '../lib/components/FavoritesView.svelte' },
  { label: 'src/lib/components/Header.svelte', path: '../lib/components/Header.svelte' },
  { label: 'src/lib/components/KeyboardDisplay.svelte', path: '../lib/components/KeyboardDisplay.svelte' },
  { label: 'src/lib/components/LibraryShare.svelte', path: '../lib/components/LibraryShare.svelte' },
  { label: 'src/lib/components/LivePlayView.svelte', path: '../lib/components/LivePlayView.svelte' },
  { label: 'src/lib/components/MidiFileList.svelte', path: '../lib/components/MidiFileList.svelte' },
  { label: 'src/lib/components/PlaybackControls.svelte', path: '../lib/components/PlaybackControls.svelte' },
  { label: 'src/lib/components/PlaylistManager.svelte', path: '../lib/components/PlaylistManager.svelte' },
  { label: 'src/lib/components/SavedPlaylistsView.svelte', path: '../lib/components/SavedPlaylistsView.svelte' },
  { label: 'src/lib/components/SearchSort.svelte', path: '../lib/components/SearchSort.svelte' },
  { label: 'src/lib/components/SettingsView.svelte', path: '../lib/components/SettingsView.svelte' },
  { label: 'src/lib/components/SongContextMenu.svelte', path: '../lib/components/SongContextMenu.svelte' },
  { label: 'src/lib/components/StatsView.svelte', path: '../lib/components/StatsView.svelte' },
  { label: 'src/lib/components/Timeline.svelte', path: '../lib/components/Timeline.svelte' },
  { label: 'src/lib/components/Visualizer.svelte', path: '../lib/components/Visualizer.svelte' },
]

describe('Svelte component smoke tests', () => {
  for (const { label, path: componentPath } of components) {
    const timeout = label === 'src/App.svelte' ? 15000 : 5000
    it(`${label} compiles`, async () => {
      const component = await bundleComponent(componentPath)
      expect(typeof component).toBe('function')
    }, timeout)
  }
})

async function bundleComponent(componentPath) {
  const absoluteComponentPath = path.resolve(testsDir, componentPath)
  const { stdout } = await execFileAsync('node', [scriptPath, absoluteComponentPath], { encoding: 'utf8' })
  return await loadModuleFromCode(stdout)
}

function generateTempFileName(base) {
  const suffix = crypto.randomUUID?.() ?? crypto.randomBytes(6).toString('hex')
  return path.join(smokeBundlesDir, `${base}-${suffix}.mjs`)
}

async function loadModuleFromCode(code) {
  await ensureBundlesDir
  const modulePath = generateTempFileName('components-smoke')
  tempFiles.add(modulePath)
  await writeFile(modulePath, code, 'utf8')
  const module = await import(pathToFileURL(modulePath).href)
  return module.default
}

function cleanupTempFilesSync() {
  if (tempFiles.size === 0) return
  for (const file of tempFiles) {
    try {
      rmSync(file, { force: true })
    } catch {
      // ignore
    }
  }
  tempFiles.clear()
}

process.on('exit', cleanupTempFilesSync)
