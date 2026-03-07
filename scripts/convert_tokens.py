#!/usr/bin/env python3
"""
转换 token 格式为 codex-tools 标准格式
"""
import json
import sys
from pathlib import Path


def convert_token_format(input_file: Path) -> dict:
    """转换 token 格式"""
    with open(input_file, 'r') as f:
        data = json.load(f)

    # 转换为 codex-tools 格式
    converted = {
        "OPENAI_API_KEY": None,
        "auth_mode": "chatgpt",
        "last_refresh": data.get("last_refresh", ""),
        "tokens": {
            "access_token": data.get("access_token", ""),
            "account_id": data.get("account_id", ""),
            "id_token": data.get("id_token", ""),
            "refresh_token": data.get("refresh_token", "")
        }
    }

    return converted


def main():
    tokens_dir = Path("tokens")
    output_dir = Path("codex_tokens")
    output_dir.mkdir(exist_ok=True)

    count = 0
    for token_file in tokens_dir.glob("token_*.json"):
        try:
            converted = convert_token_format(token_file)

            # 使用邮箱作为文件名
            email = converted["tokens"].get("id_token", "")
            if email:
                # 从 id_token 中提取邮箱（简化处理）
                import base64
                try:
                    payload = email.split('.')[1]
                    pad = '=' * ((4 - (len(payload) % 4)) % 4)
                    decoded = base64.urlsafe_b64decode((payload + pad).encode('ascii'))
                    token_data = json.loads(decoded.decode('utf-8'))
                    email = token_data.get('email', 'unknown')
                except:
                    email = token_file.stem.replace('token_', '')
            else:
                email = token_file.stem.replace('token_', '')

            output_file = output_dir / f"{email.replace('@', '_').replace('.', '_')}.json"

            with open(output_file, 'w') as f:
                json.dump(converted, f, indent=2, ensure_ascii=False)

            print(f"✅ Converted: {token_file.name} -> {output_file.name}")
            count += 1

        except Exception as e:
            print(f"❌ Failed to convert {token_file.name}: {e}")

    print(f"\n✅ Total converted: {count} files")
    print(f"📁 Output directory: {output_dir}")


if __name__ == "__main__":
    main()
