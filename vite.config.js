import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

function ensureServerEnvironmentsPlugin() {
  return {
    name: 'ensure-server-environments',
    configureServer(server) {
      if (!server.environments) {
        server.environments = {}
      }
    }
  }
}

/* set githup secret "VITEST" as `false` to skip test runs during ci. */
const isTest = process.env.VITEST === 'true' ?? true;
const defaultPlugins = [ensureServerEnvironmentsPlugin()]
if (!isTest) {
  defaultPlugins.push(svelte())
}
export default defineConfig({
  plugins: defaultPlugins,
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: 'esnext',
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/setupTests.js',
    coverage: {
        provider: 'v8',
        reporter: ['text', 'lcov'],
        all: true,
        include: ['src/lib/utils/**/*.js', 'src/lib/version.js'],
        exclude: ['**/*.svelte'],
        statements: 80,
        branches: 80,
        functions: 80,
        lines: 80
    },
  },
})