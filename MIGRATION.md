# 数据迁移说明

## 版本 0.3.0 - 与 Codex Tools 切割

### 变更内容

从版本 0.3.0 开始，Codexs 不再依赖 Codex Tools，使用自己的数据存储路径。

**旧路径（Codex Tools）：**
```
~/Library/Application Support/com.carry.codex-tools/accounts.json
```

**新路径（Codexs）：**
```
~/Library/Application Support/com.codexs.app/accounts.json
```

### 迁移步骤

如果你之前使用过 Codex Tools 并且想要迁移账号数据：

1. **备份旧数据**（可选）：
   ```bash
   cp ~/Library/Application\ Support/com.carry.codex-tools/accounts.json \
      ~/Library/Application\ Support/com.carry.codex-tools/accounts.json.backup
   ```

2. **复制到新路径**：
   ```bash
   mkdir -p ~/Library/Application\ Support/com.codexs.app
   cp ~/Library/Application\ Support/com.carry.codex-tools/accounts.json \
      ~/Library/Application\ Support/com.codexs.app/accounts.json
   ```

3. **重启 Codexs 应用**

### 注意事项

- 迁移后，Codexs 和 Codex Tools 将使用各自独立的账号数据
- 在 Codexs 中添加或删除账号不会影响 Codex Tools
- 两个应用可以同时使用，互不干扰

### 全新安装

如果你是第一次使用 Codexs，无需进行任何迁移操作。应用会自动创建新的数据目录。
