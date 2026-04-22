# Slock Desktop

Tauri desktop shell for Slock. Phase 1 focuses on a persistent skin system and a local Theme Studio that controls the remote Slock workspace window.

## Current shape

- `Theme Studio` local control window built with React + Vite
- `Workspace` remote Tauri window that loads `https://app.slock.ai`
- Three built-in themes: `Default`, `Graphite`, `Crimson`
- Theme persistence in the app config directory
- Runtime theme injection into the remote workspace
- Local service settings, start, stop, and workspace-coupled auto-start
- GitHub release check panel with one-click release and asset opening

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

- signed in-app updater flow
- service health checks and richer stack orchestration
- release workflow automation
- OS autostart
