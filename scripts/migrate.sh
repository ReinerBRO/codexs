#!/bin/bash

# Codexs 数据迁移脚本
# 从 Codex Tools 迁移账号数据到 Codexs

set -e

OLD_PATH="$HOME/Library/Application Support/com.carry.codex-tools/accounts.json"
NEW_PATH="$HOME/Library/Application Support/com.codexs.app/accounts.json"
NEW_DIR="$HOME/Library/Application Support/com.codexs.app"

echo "=== Codexs 数据迁移工具 ==="
echo ""

# 检查旧数据是否存在
if [ ! -f "$OLD_PATH" ]; then
    echo "❌ 未找到 Codex Tools 的账号数据"
    echo "   路径: $OLD_PATH"
    echo ""
    echo "如果你是第一次使用 Codexs，无需迁移。"
    exit 0
fi

echo "✓ 找到 Codex Tools 的账号数据"
echo "  路径: $OLD_PATH"
echo "  大小: $(du -h "$OLD_PATH" | cut -f1)"
echo ""

# 检查新数据是否已存在
if [ -f "$NEW_PATH" ]; then
    echo "⚠️  Codexs 的账号数据已存在"
    echo "   路径: $NEW_PATH"
    echo ""
    read -p "是否覆盖现有数据？(y/N): " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "取消迁移"
        exit 0
    fi
fi

# 创建目录
echo "创建 Codexs 数据目录..."
mkdir -p "$NEW_DIR"

# 备份旧数据
BACKUP_PATH="${OLD_PATH}.backup.$(date +%Y%m%d_%H%M%S)"
echo "备份旧数据到: $BACKUP_PATH"
cp "$OLD_PATH" "$BACKUP_PATH"

# 复制数据
echo "复制数据到 Codexs..."
cp "$OLD_PATH" "$NEW_PATH"

# 设置权限
chmod 600 "$NEW_PATH"

echo ""
echo "✅ 迁移完成！"
echo ""
echo "新数据路径: $NEW_PATH"
echo "备份路径: $BACKUP_PATH"
echo ""
echo "现在可以启动 Codexs 应用了。"
