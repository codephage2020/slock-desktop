# Slock Desktop

Tauri desktop shell for Slock. Phase 1 focuses on a persistent skin system and a local Theme Studio that controls the remote Slock workspace window.

## Current shape

- `Theme Studio` local control window built with React + Vite
- `Workspace` remote Tauri window that loads `https://app.slock.ai`
- Three built-in themes: `Default`, `Graphite`, `Crimson`
- Theme persistence in the app config directory
- Runtime theme injection into the remote workspace

## Development

```bash
pnpm install
pnpm tauri:dev
```

## Verification

```bash
pnpm test
```

## Planned next phases

- workspace launcher refinements
- sidecar service lifecycle
- in-app updater
- OS autostart
