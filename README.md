<h1 align="center">Slock Desktop</h1>

<p align="center">Slock workspace client for macOS.</p>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh.md"><kbd>中文</kbd></a>
</p>

<p align="center">
  <a href="https://github.com/codephage2020/slock-desktop/releases/tag/v0.2.9"><img alt="Version / 版本 0.2.9" src="https://img.shields.io/badge/Version%20%2F%20%E7%89%88%E6%9C%AC-0.2.9-10A37F?style=flat-square&logo=github"></a>
  <a href="https://discord.gg/JY747zGc"><img alt="Discord / 社区" src="https://img.shields.io/badge/Discord%20%2F%20%E7%A4%BE%E5%8C%BA-Join%20%2F%20%E5%8A%A0%E5%85%A5-5865F2?style=flat-square&logo=discord&logoColor=white"></a>
</p>

> [!TIP]
> Unsigned app unlock:
>
> ```bash
> sudo xattr -rd com.apple.quarantine /Applications/Slock\ Desktop.app
> ```

Slock Desktop is the macOS client for the Slock workspace at `https://app.slock.ai`. It adds desktop controls for themes, language, updates, and local Slock services.

## Features

- Opens the signed-in Slock workspace in a native macOS window.
- Provides light, dark, system, and custom accent themes.
- Applies local language and appearance settings to the workspace.
- Manages local Slock service discovery, start, stop, and auto-start.
- Stores desktop preferences in the local app config directory.

## Requirements

macOS, Node.js with `pnpm`, Rust with Cargo, Tauri macOS dependencies, and a Slock account.

## Commands

| Task | Command |
| --- | --- |
| Install dependencies | `pnpm install` |
| Desktop development | `pnpm tauri:dev` |
| Build and open the current debug app bundle | `pnpm tauri:debug:open` |
| Frontend development | `pnpm dev` |
| Project checks | `pnpm test` |
| Rust tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Build app | `pnpm build && pnpm tauri build --bundles app` |

Build output:

```text
src-tauri/target/release/bundle/macos/Slock Desktop.app
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

Store API keys, local tokens, and signed-in session data outside git. The desktop app stores local settings in the app config directory.
