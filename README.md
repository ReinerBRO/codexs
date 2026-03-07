# Codexs

<div align="center">
  <img src="app-icon.png" alt="Codexs Logo" width="200"/>

  **Codex 无限 Token 终极解决方案**

  [![Release](https://img.shields.io/github/v/release/ReinerBRO/codexs)](https://github.com/ReinerBRO/codexs/releases)
  [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
  [![Platform](https://img.shields.io/badge/platform-macOS-lightgrey)](https://github.com/ReinerBRO/codexs)

  [English](#english) | [中文](#中文) | [日本語](#日本語)
</div>

---

## 中文

### 简介

Codexs 是一个专为 Codex Tools 用户设计的桌面应用，帮助你批量生成 OpenAI 账号并一键导入到 Codex Tools，实现无限 token 自由。

### 核心功能

- ✅ **批量生成账号** - 想要多少就生成多少
- ✅ **一键导入** - 自动导入到 Codex Tools
- ✅ **智能去重** - 避免重复导入
- ✅ **实时进度** - 显示详细的生成进度（"1/10 - email@example.com 成功"）
- ✅ **多语言支持** - 中文、英文、日文
- ✅ **现代界面** - Light 风格，简洁美观
- ✅ **全选功能** - 一键全选未导入账号

### 快速开始

1. 下载最新版本的 [Codexs.dmg](https://github.com/ReinerBRO/codexs/releases/latest)
2. 拖动到 Applications 文件夹
3. 打开应用（首次打开需要右键 → 打开）
4. 输入要生成的账号数量
5. 点击"开始生成"
6. 点击"全选未导入"，然后点击"导入选中账号"

### 使用提示

- 建议一次生成 10-20 个账号
- 生成的账号会自动保存在本地
- **重要**：在 Codex Tools 中切换账号后，需要重新开启对话才能使用新账号额度（这是 Codex Tools 的会话机制限制）

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

Codexs is a desktop application designed for Codex Tools users to batch generate OpenAI accounts and import them to Codex Tools with one click, achieving unlimited token freedom.

### Core Features

- ✅ **Batch Generation** - Generate as many accounts as you need
- ✅ **One-Click Import** - Automatically import to Codex Tools
- ✅ **Smart Deduplication** - Avoid duplicate imports
- ✅ **Real-time Progress** - Display detailed generation progress ("1/10 - email@example.com Success")
- ✅ **Multi-language** - Chinese, English, Japanese
- ✅ **Modern UI** - Light theme, clean and beautiful
- ✅ **Select All** - One-click select all pending accounts

### Quick Start

1. Download the latest [Codexs.dmg](https://github.com/ReinerBRO/codexs/releases/latest)
2. Drag to Applications folder
3. Open the app (first time: right-click → Open)
4. Enter the number of accounts to generate
5. Click "Start Generation"
6. Click "Select All Pending", then click "Import Selected"

### Usage Tips

- Recommended to generate 10-20 accounts at a time
- Generated accounts are automatically saved locally
- **Important**: After switching accounts in Codex Tools, you need to restart the conversation to use the new account's quota (this is a limitation of Codex Tools' session mechanism)

### System Requirements

- macOS 11.0 or higher
- Apple Silicon (M1/M2/M3) or Intel processor
- No need to install Python or other dependencies (built-in)

### Tech Stack

- Tauri + React + TypeScript
- Rust backend
- Python scripts (packaged as standalone executable)

---

## 日本語

### 概要

Codexs は Codex Tools ユーザー向けのデスクトップアプリケーションで、OpenAI アカウントを一括生成し、ワンクリックで Codex Tools にインポートすることで、無限トークンの自由を実現します。

### 主な機能

- ✅ **一括生成** - 必要な数だけアカウントを生成
- ✅ **ワンクリックインポート** - Codex Tools に自動インポート
- ✅ **スマート重複排除** - 重複インポートを回避
- ✅ **リアルタイム進捗** - 詳細な生成進捗を表示（「1/10 - email@example.com 成功」）
- ✅ **多言語対応** - 中国語、英語、日本語
- ✅ **モダン UI** - ライトテーマ、シンプルで美しい
- ✅ **全選択** - 未インポートアカウントをワンクリックで全選択

### クイックスタート

1. 最新版の [Codexs.dmg](https://github.com/ReinerBRO/codexs/releases/latest) をダウンロード
2. Applications フォルダにドラッグ
3. アプリを開く（初回：右クリック → 開く）
4. 生成するアカウント数を入力
5. 「生成開始」をクリック
6. 「未インポートを全選択」をクリックし、「選択したアカウントをインポート」をクリック

### 使用上のヒント

- 一度に 10-20 個のアカウントを生成することをお勧めします
- 生成されたアカウントは自動的にローカルに保存されます
- **重要**：Codex Tools でアカウントを切り替えた後、新しいアカウントの割り当てを使用するには会話を再開する必要があります（これは Codex Tools のセッションメカニズムの制限です）

### システム要件

- macOS 11.0 以降
- Apple Silicon (M1/M2/M3) または Intel プロセッサ
- Python やその他の依存関係のインストールは不要（内蔵）

### 技術スタック

- Tauri + React + TypeScript
- Rust バックエンド
- Python スクリプト（スタンドアロン実行可能ファイルとしてパッケージ化）

---

## License

MIT License

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Disclaimer

This tool is for educational and research purposes only. Please comply with OpenAI's Terms of Service and do not abuse the batch registration feature.
