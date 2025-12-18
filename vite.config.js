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

export default defineConfig(({ mode }) => {
  const isTest = mode === 'test'
  const plugins = [ensureServerEnvironmentsPlugin()]
  if (!isTest) {
    plugins.push(svelte())
  }

  return {
    plugins,
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
        include: ['src/**/*.{js,ts,svelte}', 'src-tauri/**/*.{js,ts,svelte}'],
        exclude: [
          'node_modules/**',
          'src/lib/components/**',
          'src/lib/stores/band.js',
          'src/lib/stores/library.js',
          'src/lib/stores/player.js',
        ],
      },
    },
  }
})
