# Codexs

<div align="center">
  <img src="app-icon.png" alt="Codexs Logo" width="200"/>

  **Codex 账号管理与无限 Token 解决方案**

  [![Release](https://img.shields.io/github/v/release/ReinerBRO/codexs)](https://github.com/ReinerBRO/codexs/releases)
  [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)](https://github.com/ReinerBRO/codexs)

  [English](#english) | [中文](#中文) | [日本語](#日本語)
</div>

---

## 中文

### 简介

Codexs 是一个独立的 Codex 账号管理工具，提供批量生成账号、多账号管理、智能切换、SSH 远程同步等功能，让你的 Codex 使用体验更加流畅。

**v0.3.0 重要更新**：Codexs 现已完全独立，不再依赖 Codex Tools。如果你之前使用过 Codex Tools，请查看 [数据迁移说明](./MIGRATION.md)。

### 核心功能

#### 账号生成
- ✅ **批量生成账号** - 想要多少就生成多少
- ✅ **智能去重** - 避免重复导入
- ✅ **实时进度** - 显示详细的生成进度

#### 账号管理
- ✅ **多账号管理** - 一键切换 Codex 账号
- ✅ **用量监控** - 实时显示 5小时/1周 用量窗口
- ✅ **智能切换** - 自动选择用量最低的账号
- ✅ **自定义标签** - 为每个账号设置易记的标签

#### 终端会话管理
- ✅ **自动恢复** - 切换账号时自动恢复 CLI 会话
- ✅ **智能检测** - 支持 iTerm2 和 Terminal，跳过 VSCode 终端
- ✅ **原地恢复** - 在原终端标签页中恢复，无需手动操作

#### SSH 远程同步
- ✅ **一键同步** - 同步认证信息到远程服务器
- ✅ **自动同步** - 切换账号时自动同步（可选）
- ✅ **配置导入** - 从 ~/.ssh/config 导入服务器配置

#### 其他特性
- ✅ **多语言支持** - 中文、英文、日文
- ✅ **现代界面** - Light 风格，简洁美观
- ✅ **自动刷新** - 每 30 秒自动更新账号用量

### 快速开始

1. 下载最新版本的 [Codexs.dmg](https://github.com/ReinerBRO/codexs/releases/latest)
2. 拖动到 Applications 文件夹
3. 打开应用（首次打开需要右键 → 打开）
4. 如果你之前使用过 Codex Tools，运行迁移脚本（可选）：
   ```bash
   /Applications/Codexs.app/Contents/Resources/scripts/migrate.sh
   ```
5. 开始使用！

### 数据迁移（从 Codex Tools）

如果你之前使用过 Codex Tools，可以使用自动迁移脚本：

```bash
/Applications/Codexs.app/Contents/Resources/scripts/migrate.sh
```

或者手动迁移：

```bash
mkdir -p ~/Library/Application\ Support/com.codexs.app
cp ~/Library/Application\ Support/com.carry.codex-tools/accounts.json \
   ~/Library/Application\ Support/com.codexs.app/accounts.json
```

详细说明请查看 [MIGRATION.md](./MIGRATION.md)。

### 使用提示

- 建议一次生成 10-20 个账号
- 生成的账号会自动保存在本地
- 开启"自动恢复 CLI 会话"可以在切换账号时自动恢复终端会话
- 配置 SSH 服务器后，可以实现本地切换、远程自动同步

### 系统要求

- macOS 11.0 或更高版本
- Apple Silicon (M1/M2/M3) 或 Intel 处理器
- 无需安装 Python 或其他依赖（已内置）

### 技术栈

- Tauri + React + TypeScript
- Rust 后端
- Python 脚本（已打包为独立可执行文件）

---

## English

### Introduction

Codexs is an independent Codex account management tool that provides batch account generation, multi-account management, smart switching, SSH remote sync, and more to make your Codex experience smoother.

**v0.3.0 Important Update**: Codexs is now completely independent and no longer depends on Codex Tools. If you previously used Codex Tools, please see the [Migration Guide](./MIGRATION.md).

### Core Features

#### Account Generation
- ✅ **Batch Generation** - Generate as many accounts as you need
- ✅ **Smart Deduplication** - Avoid duplicate imports
- ✅ **Real-time Progress** - Display detailed generation progress

#### Account Management
- ✅ **Multi-account Management** - Switch Codex accounts with one click
- ✅ **Usage Monitoring** - Real-time display of 5-hour/1-week usage windows
- ✅ **Smart Switching** - Automatically select the account with the lowest usage
- ✅ **Custom Labels** - Set memorable labels for each account

#### Terminal Session Management
- ✅ **Auto-resume** - Automatically resume CLI sessions when switching accounts
- ✅ **Smart Detection** - Supports iTerm2 and Terminal, skips VSCode terminal
- ✅ **In-place Recovery** - Resume in the original terminal tab, no manual operation required

#### SSH Remote Sync
- ✅ **One-click Sync** - Sync authentication info to remote servers
- ✅ **Auto-sync** - Automatically sync when switching accounts (optional)
- ✅ **Config Import** - Import server config from ~/.ssh/config

#### Other Features
- ✅ **Multi-language** - Chinese, English, Japanese
- ✅ **Modern UI** - Light theme, clean and beautiful
- ✅ **Auto-refresh** - Automatically update account usage every 30 seconds

### Quick Start

1. Download the latest [Codexs.dmg](https://github.com/ReinerBRO/codexs/releases/latest)
2. Drag to Applications folder
3. Open the app (first time: right-click → Open)
4. If you previously used Codex Tools, run the migration script (optional):
   ```bash
   /Applications/Codexs.app/Contents/Resources/scripts/migrate.sh
   ```
5. Start using!

### Data Migration (from Codex Tools)

If you previously used Codex Tools, you can use the automatic migration script:

```bash
/Applications/Codexs.app/Contents/Resources/scripts/migrate.sh
```

Or migrate manually:

```bash
mkdir -p ~/Library/Application\ Support/com.codexs.app
cp ~/Library/Application\ Support/com.carry.codex-tools/accounts.json \
   ~/Library/Application\ Support/com.codexs.app/accounts.json
```

For detailed instructions, see [MIGRATION.md](./MIGRATION.md).

### Usage Tips

- Recommended to generate 10-20 accounts at a time
- Generated accounts are automatically saved locally
- Enable "Auto-resume CLI sessions" to automatically resume terminal sessions when switching accounts
- After configuring SSH servers, you can achieve local switching and remote auto-sync

### System Requirements

- macOS 11.0 or higher
- Apple Silicon (M1/M2/M3) or Intel processor
- No need to install Python or other dependencies (built-in)

### Tech Stack

- Tauri + React + TypeScript
- Rust backend
- Python scripts (packaged as standalone executables)

---

## 日本語

### 概要

Codexs は独立した Codex アカウント管理ツールで、バッチアカウント生成、マルチアカウント管理、スマート切り替え、SSH リモート同期などの機能を提供し、Codex の使用体験をよりスムーズにします。

**v0.3.0 重要な更新**: Codexs は完全に独立し、Codex Tools に依存しなくなりました。以前に Codex Tools を使用していた場合は、[移行ガイド](./MIGRATION.md) を参照してください。

### コア機能

#### アカウント生成
- ✅ **バッチ生成** - 必要な数だけアカウントを生成
- ✅ **スマート重複排除** - 重複インポートを回避
- ✅ **リアルタイム進捗** - 詳細な生成進捗を表示

#### アカウント管理
- ✅ **マルチアカウント管理** - ワンクリックで Codex アカウントを切り替え
- ✅ **使用量監視** - 5時間/1週間の使用量ウィンドウをリアルタイム表示
- ✅ **スマート切り替え** - 使用量が最も少ないアカウントを自動選択
- ✅ **カスタムラベル** - 各アカウントに覚えやすいラベルを設定

#### ターミナルセッション管理
- ✅ **自動再開** - アカウント切り替え時に CLI セッションを自動再開
- ✅ **スマート検出** - iTerm2 と Terminal をサポート、VSCode ターミナルをスキップ
- ✅ **その場で復元** - 元のターミナルタブで再開、手動操作不要

#### SSH リモート同期
- ✅ **ワンクリック同期** - 認証情報をリモートサーバーに同期
- ✅ **自動同期** - アカウント切り替え時に自動同期（オプション）
- ✅ **設定インポート** - ~/.ssh/config からサーバー設定をインポート

#### その他の機能
- ✅ **多言語対応** - 中国語、英語、日本語
- ✅ **モダンUI** - ライトテーマ、クリーンで美しい
- ✅ **自動更新** - 30秒ごとにアカウント使用量を自動更新

### クイックスタート

1. 最新の [Codexs.dmg](https://github.com/ReinerBRO/codexs/releases/latest) をダウンロード
2. Applications フォルダにドラッグ
3. アプリを開く（初回：右クリック → 開く）
4. 以前に Codex Tools を使用していた場合は、移行スクリプトを実行（オプション）：
   ```bash
   /Applications/Codexs.app/Contents/Resources/scripts/migrate.sh
   ```
5. 使用開始！

### データ移行（Codex Tools から）

以前に Codex Tools を使用していた場合は、自動移行スクリプトを使用できます：

```bash
/Applications/Codexs.app/Contents/Resources/scripts/migrate.sh
```

または手動で移行：

```bash
mkdir -p ~/Library/Application\ Support/com.codexs.app
cp ~/Library/Application\ Support/com.carry.codex-tools/accounts.json \
   ~/Library/Application\ Support/com.codexs.app/accounts.json
```

詳細な手順については、[MIGRATION.md](./MIGRATION.md) を参照してください。

### 使用のヒント

- 一度に 10-20 個のアカウントを生成することをお勧めします
- 生成されたアカウントは自動的にローカルに保存されます
- 「CLI セッションを自動再開」を有効にすると、アカウント切り替え時にターミナルセッションが自動的に再開されます
- SSH サーバーを設定すると、ローカル切り替えとリモート自動同期が実現できます

### システム要件

- macOS 11.0 以上
- Apple Silicon (M1/M2/M3) または Intel プロセッサ
- Python やその他の依存関係をインストールする必要はありません（組み込み済み）

### 技術スタック

- Tauri + React + TypeScript
- Rust バックエンド
- Python スクリプト（スタンドアロン実行可能ファイルとしてパッケージ化）

---

## License

MIT License - see [LICENSE](LICENSE) for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

If you encounter any issues or have questions, please [open an issue](https://github.com/ReinerBRO/codexs/issues).
