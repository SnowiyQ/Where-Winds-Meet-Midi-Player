import fs from 'node:fs'
import path from 'node:path'
import esbuild from 'esbuild'
import { compile } from 'svelte/compiler'

const componentPath = process.argv[2]
if (!componentPath) {
  console.error('You must pass the component path to this script')
  process.exit(1)
}

const sveltePlugin = {
  name: 'components-smoke-svelte',
  setup(build) {
    build.onResolve({ filter: /\.svelte$/ }, (args) => {
      const cleaned = args.path.split('?')[0]
      const baseDir = args.resolveDir || process.cwd()
      let resolved
      if (path.isAbsolute(cleaned)) {
        resolved = cleaned
      } else {
        const candidate = path.resolve(baseDir, cleaned)
        if (fs.existsSync(candidate)) {
          resolved = candidate
        } else {
          const rootCandidate = path.resolve(process.cwd(), cleaned)
          resolved = fs.existsSync(rootCandidate) ? rootCandidate : candidate
        }
      }
      return {
        path: resolved,
        namespace: 'svelte',
        pluginData: { resolveDir: path.dirname(resolved) }
      }
    })

    build.onResolve({ filter: /\.css$/ }, (args) => {
      const resolved = path.resolve(args.resolveDir, args.path)
      return {
        path: resolved,
        namespace: 'css',
        pluginData: { resolveDir: path.dirname(resolved) }
      }
    })

    const assetFilter = /(\.(png|jpe?g|gif|svg|webp|ico)(\?.*)?)$/i
    build.onResolve({ filter: assetFilter }, (args) => ({
      path: path.resolve(args.resolveDir, args.path),
      namespace: 'asset'
    }))
    build.onLoad({ filter: /.*/, namespace: 'asset' }, () => ({
      contents: 'export default ""',
      loader: 'js'
    }))

    const tauriFilter = /^@tauri-apps\/.+/i
    build.onResolve({ filter: tauriFilter }, (args) => ({
      path: args.path,
      namespace: 'tauri'
    }))
    build.onLoad({ filter: /.*/, namespace: 'tauri' }, (args) => {
      let contents = ''
      switch (args.path) {
        case '@tauri-apps/api/core':
          contents = 'export async function invoke() { return {} }\n'
          break
        case '@tauri-apps/api/event':
          contents = 'export async function listen() { return { unlisten: async () => null } }\nexport async function emit() { return {} }\n'
          break
        case '@tauri-apps/api/window':
          contents = `export function getCurrentWindow() { return { setMinSize: async () => null, setSize: async () => null, setFullscreen: async () => null } }\nexport class LogicalSize { constructor(width = 0, height = 0) { this.width = width; this.height = height } }\n`
          break
        case '@tauri-apps/api/dpi':
          contents = 'export class PhysicalPosition { constructor(x = 0, y = 0) { this.x = x; this.y = y } }\nexport class PhysicalSize { constructor(width = 0, height = 0) { this.width = width; this.height = height } }\n'
          break
        case '@tauri-apps/plugin-dialog':
          contents = 'export async function open() { return null }\nexport async function save() { return null }\n'
          break
        default:
          contents = 'export const __tauri_stub = true'
      }
      return { contents, loader: 'js' }
    })

    const iconifyFilter = /^@iconify\/svelte$/
    build.onResolve({ filter: iconifyFilter }, () => ({
      path: '@iconify/svelte',
      namespace: 'iconify'
    }))
    build.onLoad({ filter: /.*/, namespace: 'iconify' }, () => ({
      contents: 'class Icon {}\nexport { Icon }\nexport default Icon\n',
      loader: 'js'
    }))

    build.onLoad({ filter: /.*/, namespace: 'svelte' }, (args) => {
      const source = fs.readFileSync(args.path, 'utf8')
      const { js } = compile(source, {
        filename: args.path,
        generate: 'ssr',
        hydratable: true,
        css: 'external',
        dev: true,
      })
      return { contents: js.code, loader: 'js', resolveDir: path.dirname(args.path) }
    })

    build.onLoad({ filter: /.*/, namespace: 'css' }, () => ({ contents: '', loader: 'js' }))
  }
}

async function buildComponent() {
  const entryContents = `import Component from ${JSON.stringify(componentPath)}; export default Component;`
  const bundle = await esbuild.build({
    stdin: {
      contents: entryContents,
      resolveDir: path.dirname(componentPath),
      sourcefile: 'components-smoke-entry.mjs'
    },
    bundle: true,
    format: 'esm',
    platform: 'neutral',
    write: false,
    plugins: [sveltePlugin],
    external: ['svelte', 'svelte/internal', 'svelte/store', 'peerjs', 'is-mergeable-object'],
    conditions: ['svelte'],
    mainFields: ['svelte', 'module', 'main'],
  })
  const code = bundle.outputFiles[0].text
  process.stdout.write(code)
}

buildComponent().catch((error) => {
  console.error(error)
  process.exit(1)
})
