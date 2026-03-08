import json
import os

# Test reading from Codex Tools location
codex_tools_path = os.path.expanduser("~/Library/Application Support/com.carry.codex-tools/accounts.json")

print(f"Checking path: {codex_tools_path}")
print(f"File exists: {os.path.exists(codex_tools_path)}")

if os.path.exists(codex_tools_path):
    with open(codex_tools_path, 'r') as f:
        data = json.load(f)
        print(f"Total accounts in file: {len(data.get('accounts', []))}")
        if data.get('accounts'):
            print(f"First account: {data['accounts'][0].get('email', 'N/A')}")
