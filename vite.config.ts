import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: [
    'VITE_',
    'TAURI_PLATFORM',
    'TAURI_ARCH',
    'TAURI_FAMILY',
    'TAURI_PLATFORM_VERSION',
    'TAURI_PLATFORM_TYPE',
    'TAURI_DEBUG',
  ],
  build: {
    target: 'es2020',
    minify: process.env.TAURI_DEBUG ? false : 'esbuild',
    sourcemap: Boolean(process.env.TAURI_DEBUG),
  },
})
