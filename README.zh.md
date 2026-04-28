<h1 align="center">Slock Desktop</h1>

<p align="center">Slock 工作区 macOS 客户端。</p>

<p align="center">
  <a href="README.md"><kbd>English</kbd></a>
  <a href="README.zh.md"><kbd>中文</kbd></a>
</p>

<p align="center">
  <a href="https://github.com/codephage2020/slock-desktop/releases/tag/v0.2.4"><img alt="Version / 版本 0.2.4" src="https://img.shields.io/badge/Version%20%2F%20%E7%89%88%E6%9C%AC-0.2.4-10A37F?style=flat-square&logo=github"></a>
  <a href="https://discord.gg/JY747zGc"><img alt="Discord / 社区" src="https://img.shields.io/badge/Discord%20%2F%20%E7%A4%BE%E5%8C%BA-Join%20%2F%20%E5%8A%A0%E5%85%A5-5865F2?style=flat-square&logo=discord&logoColor=white"></a>
</p>

> [!TIP]
> 未签名应用解除隔离：
>
> ```bash
> sudo xattr -rd com.apple.quarantine /Applications/Slock\ Desktop.app
> ```

Slock Desktop 是 Slock 工作区的 macOS 桌面客户端，内置主题、语言、更新和本地服务管理。

## 功能

- 在原生 macOS 窗口中打开已登录的 Slock 工作区。
- 提供浅色、深色、跟随系统和自定义强调色主题。
- 将本地语言和外观设置应用到工作区。
- 管理本地 Slock 服务发现、启动、停止和自动启动。
- 将桌面偏好设置保存在本地 app 配置目录中。

## 环境

需要 macOS、Node.js 和 `pnpm`、Rust 和 Cargo、Tauri macOS 依赖，以及 Slock 账号。

## 命令

| 任务 | 命令 |
| --- | --- |
| 安装依赖 | `pnpm install` |
| 桌面开发 | `pnpm tauri:dev` |
| 前端开发 | `pnpm dev` |
| 项目检查 | `pnpm test` |
| Rust 测试 | `cargo test --manifest-path src-tauri/Cargo.toml` |
| 构建应用 | `pnpm build && pnpm tauri build --bundles app` |

构建产物：

```text
src-tauri/target/release/bundle/macos/Slock Desktop.app
```

## 项目结构

```text
src/                 React 桌面启动器
src/lib/desktop.ts   Tauri 命令桥接
src-tauri/           Rust Tauri 应用
src-tauri/src/       桌面状态、服务、主题和工作区逻辑
src-tauri/icons/     应用图标
```

## 安全

API key、本地 token 和登录会话数据放在 git 之外。桌面应用把本地设置保存在 app 配置目录中。
