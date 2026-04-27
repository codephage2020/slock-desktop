# Slock Desktop

Slock Desktop is the macOS client for the Slock workspace at `https://app.slock.ai`. It adds desktop controls for themes, language, updates, and local Slock services.

Slock Desktop 是 Slock 工作区的 macOS 桌面客户端，内置主题、语言、更新和本地服务管理。

- Version / 版本: `0.0.5`
- Discord / 社区: [discord.gg/JY747zGc](https://discord.gg/JY747zGc)

## Requirements / 环境

macOS, Node.js with `pnpm`, Rust with Cargo, Tauri macOS dependencies, and a Slock account.

需要 macOS、Node.js 和 `pnpm`、Rust 和 Cargo、Tauri macOS 依赖，以及 Slock 账号。

## Commands / 命令

| Task / 任务 | Command / 命令 |
| --- | --- |
| Install / 安装依赖 | `pnpm install` |
| Desktop dev / 桌面开发 | `pnpm tauri:dev` |
| Frontend dev / 前端开发 | `pnpm dev` |
| Checks / 项目检查 | `pnpm test` |
| Rust tests / Rust 测试 | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Build app / 构建应用 | `pnpm build && pnpm tauri build --bundles app` |

Build output / 构建产物:

```text
src-tauri/target/release/bundle/macos/Slock Desktop.app
```

## Release / 发布

- Builds / 发布包: [GitHub Releases](https://github.com/codephage2020/slock-desktop/releases)
- Unsigned app unlock / 未签名应用解除隔离: `sudo xattr -rd com.apple.quarantine /Applications/Slock\ Desktop.app`
- Version files / 版本文件: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
- Updater manifest / 更新清单: `https://github.com/codephage2020/slock-desktop/releases/latest/download/latest.json`

For signed updater builds, generate a key with `pnpm tauri signer generate -w ~/.tauri/slock-desktop.key`, set `SLOCK_DESKTOP_UPDATER_PUBKEY` and `TAURI_SIGNING_PRIVATE_KEY`, then run checks and build.

签名更新包需要先生成 Tauri updater key，设置 `SLOCK_DESKTOP_UPDATER_PUBKEY` 和 `TAURI_SIGNING_PRIVATE_KEY`，再运行检查和构建。

## Project / 项目

```text
src/                 React desktop launcher
src/lib/desktop.ts   Tauri command bridge
src-tauri/           Rust Tauri application
src-tauri/src/       Desktop state, service, theme, and workspace logic
src-tauri/icons/     App icons
```

## Security / 安全

Store API keys, local tokens, and signed-in session data outside git. The desktop app stores local settings in the app config directory.

API key、本地 token 和登录会话数据放在 git 之外。桌面应用把本地设置保存在 app 配置目录中。
