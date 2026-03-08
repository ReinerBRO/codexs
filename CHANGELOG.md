# Changelog

All notable changes to this project will be documented in this file.

## v0.3.0 - 2026-03-08

### 🎉 Major Changes

- **独立于 Codex Tools**：不再依赖 Codex Tools，使用自己的数据存储路径
  - 新路径：`~/Library/Application Support/com.codexs.app/accounts.json`
  - 旧路径：`~/Library/Application Support/com.carry.codex-tools/accounts.json`
  - 详见 [MIGRATION.md](./MIGRATION.md) 了解如何迁移数据

### ✨ Features

- 添加账号管理界面的三语支持（中文、英文、日文）
- 添加 SSH 自动同步功能（切换账号时自动同步到远程服务器）
- 添加智能切换功能（自动选择用量最低的账号）
- 添加 VSCode 终端检测，跳过自动恢复（不打断工作流）
- 添加账号用量自动刷新（每 30 秒）

### 🐛 Bug Fixes

- 修复账号管理标题硬编码中文的问题
- 修复多个 UI 显示问题

### 📝 Documentation

- 添加数据迁移说明文档
- 更新推广文案（小红书、知乎）

## v0.1.0 - 2026-03-07

- Initial release
- Added macOS desktop shell based on Tauri v2
- Added batch account generation and Codex Tools import workflow
- Added GitHub Actions release automation for macOS Apple Silicon and Intel builds
