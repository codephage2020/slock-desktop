# Slock Desktop

Slock Desktop is a Tauri app for running the Slock workspace as a desktop application. It opens `https://app.slock.ai`, applies a desktop theme layer, and gives the user local controls for themes, language, updates, and Slock server daemons.

Current version: `0.0.1`

## Features

- React + Vite launcher for desktop settings and workspace entry.
- Tauri shell that opens the signed-in Slock workspace in the main window.
- Built-in themes, custom accent themes, light/dark/system modes, and language settings.
- Runtime CSS and script injection for the Slock workspace.
- Local server daemon discovery, start, stop, and workspace auto-start.
- Close-app behavior settings for keeping or stopping local server daemons.
- GitHub release check panel with release-page and asset links.

## Requirements

- macOS for the current packaged desktop build.
- Node.js with `pnpm`.
- Rust toolchain with Cargo.
- Tauri system dependencies for macOS.
- A Slock account with access to `https://app.slock.ai`.

## Setup

```bash
pnpm install
```

Start the desktop app in development mode:

```bash
pnpm tauri:dev
```

Run the Vite frontend by itself:

```bash
pnpm dev
```

## Verification

Run the project checks:

```bash
pnpm test
```

Run Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Build

Build the frontend:

```bash
pnpm build
```

Build the macOS `.app` bundle:

```bash
pnpm tauri build --bundles app
```

The app bundle is written to:

```text
src-tauri/target/release/bundle/macos/Slock Desktop.app
```

## Releases

Published builds live on the GitHub Releases page:

```text
https://github.com/codephage2020/slock-tauri/releases
```

For a release build, set the version in all three files:

```text
package.json
src-tauri/Cargo.toml
src-tauri/tauri.conf.json
```

Then run:

```bash
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build --bundles app
```

## Project Layout

```text
src/                 React desktop launcher
src/lib/desktop.ts   Tauri command bridge
src-tauri/           Rust Tauri application
src-tauri/src/       Desktop state, service, theme, and workspace logic
src-tauri/icons/     App icons
```

## Security

This is a public repository. Keep API keys, local machine tokens, and signed-in session data out of git. The desktop app stores local settings in the app config directory and reads authenticated Slock state from the user session.
