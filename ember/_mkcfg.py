#!/usr/bin/env python3
"""一次性辅助脚本：从环境变量 APIHUB_AGNES_AI_API_KEY 生成 config.json（权限 600）。"""
import json
import os

here = os.path.dirname(os.path.abspath(__file__))
key = os.environ.get("APIHUB_AGNES_AI_API_KEY", "").strip()
if not key:
    raise SystemExit("环境变量 APIHUB_AGNES_AI_API_KEY 未设置，无法生成 config.json")
cfg = {"base_url": "https://api.agnes-ai.cn/v1", "model": "agnes-3.0-flash", "key": key}
path = os.path.join(here, "config.json")
with open(path, "w", encoding="utf-8") as f:
    f.write(json.dumps(cfg, ensure_ascii=False, indent=2))
os.chmod(path, 0o600)
print("config.json written (mode 600), model = %s" % cfg["model"])
