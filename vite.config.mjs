import babel from '@rolldown/plugin-babel';
import react, { reactCompilerPreset } from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';

const COVERAGE_THRESHOLD = 90;
const UNDER_VITEST = process.env.VITEST !== undefined;
const TAURI_DEV_HOST = process.env.TAURI_DEV_HOST;
const TAURI_DEV_PORT = 1420;
const TAURI_HMR_PORT = 1421;

const reactCompiler = babel({
  presets: [reactCompilerPreset({ panicThreshold: 'all_errors' })],
});

export default defineConfig({
  plugins: UNDER_VITEST ? [react()] : [react(), reactCompiler],
  clearScreen: false,
  envPrefix: ['VITE_', 'TAURI_ENV_'],
  server: {
    port: TAURI_DEV_PORT,
    strictPort: true,
    host: TAURI_DEV_HOST ?? false,
    hmr: TAURI_DEV_HOST
      ? { protocol: 'ws', host: TAURI_DEV_HOST, port: TAURI_HMR_PORT }
      : undefined,
    watch: { ignored: ['**/crates/**', '**/xtask/**', '**/target/**', '**/docs/**'] },
  },
  build: { sourcemap: true, target: 'es2024' },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.spec.{ts,tsx}'],
    setupFiles: ['vitest.setup.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/**/*.{ts,tsx}'],
      exclude: ['src/**/*.spec.{ts,tsx}', 'src/main.tsx'],
      reporter: ['text-summary', 'json'],
      thresholds: {
        lines: COVERAGE_THRESHOLD,
        branches: COVERAGE_THRESHOLD,
        functions: COVERAGE_THRESHOLD,
        statements: COVERAGE_THRESHOLD,
      },
    },
  },
});
