# Slock Desktop

Tauri desktop shell for Slock. The desktop console opens the original `app.slock.ai` workspace in the main window, injects a persistent theme system, and adds a desktop settings surface inside the workspace page.

## Current shape

- `Desktop Console` local launch surface built with React + Vite
- Main Tauri window navigates directly to `https://app.slock.ai` after launch
- Five built-in themes: `Default`, `Light`, `Dark`, `Graphite`, `Rose`
- Theme persistence in the app config directory
- Runtime theme injection into the workspace page
- Injected `Desktop Settings` panel for in-workspace theme switching
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
