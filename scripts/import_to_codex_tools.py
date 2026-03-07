#!/usr/bin/env python3
"""
将 token 导入到 Codex Tools 的 accounts.json
"""
import argparse
import json
import uuid
import time
from pathlib import Path
from typing import Optional, Set


def import_to_codex_tools(selected_emails: Optional[Set[str]] = None) -> int:
    """导入 token 到 Codex Tools"""
    # Codex Tools 的 accounts.json 路径
    accounts_file = Path.home() / "Library/Application Support/com.carry.codex-tools/accounts.json"

    # 读取现有账号
    if accounts_file.exists():
        with open(accounts_file, 'r') as f:
            data = json.load(f)

        # 备份
        backup_file = Path("codex_tools_accounts_backup.json")
        with open(backup_file, 'w') as f:
            json.dump(data, f, indent=2)
        print(f"📦 已备份到: {backup_file}")
    else:
        data = {
            "version": 1,
            "accounts": []
        }

    # 读取转换后的 token
    codex_tokens_dir = Path("codex_tokens")
    if not codex_tokens_dir.exists():
        print("❌ codex_tokens 目录不存在，请先运行 convert_tokens.py")
        print(
            "SUMMARY_JSON:"
            + json.dumps(
                {
                    "added": [],
                    "skipped_existing": [],
                    "failed_files": [],
                },
                ensure_ascii=False,
            )
        )
        return 1

    existing_emails = {acc["email"] for acc in data["accounts"]}
    added_count = 0
    added_emails = []
    skipped_existing = []
    failed_files = []

    for token_file in codex_tokens_dir.glob("*.json"):
        try:
            with open(token_file, 'r') as f:
                auth_json = json.load(f)

            # 从 id_token 提取邮箱
            id_token = auth_json["tokens"]["id_token"]
            import base64
            payload = id_token.split('.')[1]
            pad = '=' * ((4 - (len(payload) % 4)) % 4)
            decoded = base64.urlsafe_b64decode((payload + pad).encode('ascii'))
            jwt_data = json.loads(decoded.decode('utf-8'))

            email = str(jwt_data.get('email', '')).strip().lower()
            account_id = auth_json["tokens"]["account_id"]
            plan_type = jwt_data.get('https://api.openai.com/auth', {}).get('chatgpt_plan_type', 'free')

            if selected_emails is not None and email not in selected_emails:
                continue

            # 跳过已存在的账号
            if email in existing_emails:
                print(f"⏭️  跳过已存在: {email}")
                skipped_existing.append(email)
                continue

            # 创建新账号条目
            new_account = {
                "id": str(uuid.uuid4()),
                "label": email,
                "email": email,
                "accountId": account_id,
                "planType": plan_type,
                "authJson": auth_json,
                "addedAt": int(time.time()),
                "updatedAt": int(time.time()),
                "usage": None,
                "usageError": None
            }

            data["accounts"].append(new_account)
            print(f"✅ 添加: {email} ({plan_type})")
            added_count += 1
            added_emails.append(email)
            existing_emails.add(email)

        except Exception as e:
            print(f"❌ 导入失败 {token_file.name}: {e}")
            failed_files.append(token_file.name)

    # 保存更新后的 accounts.json
    if added_count > 0:
        accounts_file.parent.mkdir(parents=True, exist_ok=True)
        with open(accounts_file, 'w') as f:
            json.dump(data, f, indent=2, ensure_ascii=False)

        print(f"\n✅ 成功导入 {added_count} 个新账号")
        print(f"📁 已保存到: {accounts_file}")
        print(f"📊 总账号数: {len(data['accounts'])}")
        print("\n现在:")
        print("1. 打开或重启 Codex Tools 应用")
        print("2. 应该能看到所有账号")
        print("3. 点击「刷新」查看用量")
    else:
        print("\n⚠️  没有新账号需要导入")

    print(
        "SUMMARY_JSON:"
        + json.dumps(
            {
                "added": added_emails,
                "skipped_existing": skipped_existing,
                "failed_files": failed_files,
            },
            ensure_ascii=False,
        )
    )

    return 0 if not failed_files else 2


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="导入 codex_tokens 到 Codex Tools")
    parser.add_argument("--emails", nargs="*", help="只导入指定邮箱")
    args = parser.parse_args()

    selected = None
    if args.emails:
        selected = {str(email).strip().lower() for email in args.emails if str(email).strip()}

    raise SystemExit(import_to_codex_tools(selected))
