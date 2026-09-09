#!/usr/bin/env python3
"""Check the running service process env for provider keys (masked)."""
import paramiko

c = paramiko.SSHClient()
c.set_missing_host_key_policy(paramiko.AutoAddPolicy())
c.connect("192.168.220.131", username="wutao", password="", timeout=30)

cmd = (
    "pid=$(pgrep -f 'target/debug/service' | head -1); "
    "echo pid=$pid; "
    "tr '\\0' '\\n' < /proc/$pid/environ | grep -iE 'zhipu|deepseek|glm|key|token' "
    "| sed 's/=.*/=SET/'"
)
_, o, e = c.exec_command(cmd, timeout=30)
print(o.read().decode(errors="replace"))
print(e.read().decode(errors="replace"))
