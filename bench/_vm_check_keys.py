#!/usr/bin/env python3
"""Find how the service gets provider credentials (config file vs env)."""
import paramiko

c = paramiko.SSHClient()
c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect("192.168.220.131", username="wutao", password=os.environ.get("CODEX_VM_PW", ""), timeout=30)

cmds = [
    "ls ~/codex_work/*.toml ~/codex_work/.env ~/codex_work/config* 2>/dev/null",
    "grep -rl 'api_key\\|apikey\\|API_KEY' ~/codex_work --include='*.toml' --include='*.json' --include='.env' 2>/dev/null | grep -v target | grep -v crates | head",
    "cat ~/codex_work/providers.toml 2>/dev/null | sed 's/key *=.*/key = MASKED/' | head -30",
    "ls ~/codex_work/ | head -30",
]
for cmd in cmds:
    _, o, e = c.exec_command(cmd, timeout=30)
    print("$", cmd)
    print(o.read().decode(errors="replace"))
